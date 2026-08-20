"use client";

import { useMemo, useState } from "react";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Input,
  Button,
} from "@/components";
import { ChevronLeft, ChevronRight, Download, Search } from "lucide-react";
import { SortBy, useGetValidatorsTable, ValidatorsTableRow } from "@/hooks";
import { useDebounceCallback } from "usehooks-ts";
import { roundDecimals } from "@/lib/helpers";

const downloadCsvData = (rawData: ValidatorsTableRow[]) => {
  const data = rawData.map(
    ({ voteDate, voterSplits, name, percentage, activated_stake }) => ({
      name,
      activated_stake,
      yes: voterSplits.yes,
      no: voterSplits.no,
      abstain: voterSplits.abstain,
      undecided: voterSplits.undecided,
      percentage,
      voteDate,
    }),
  );
  if (!data || data.length === 0) return;

  const headers = Object.keys(data[0]) as (keyof (typeof data)[0])[];
  const csvRows = [
    headers.join(","), // header row
    ...data.map((row) =>
      headers
        .map((field) => {
          const value = row[field];
          if (typeof value === "string") {
            // Escape quotes
            return `"${value.replace(/"/g, '""')}"`;
          }
          return value;
        })
        .join(","),
    ),
  ];

  const csvContent = csvRows.join("\n");
  const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8;" });

  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.setAttribute("download", "top-voters.csv");
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
};

export function TopVoters() {
  const [searchQuery, setSearchQuery] = useState("");
  const [currentPage, setCurrentPage] = useState(1);
  const [itemsPerPage, setItemsPerPage] = useState(10);
  const [sortBy, setSortBy] = useState<SortBy>("weight");

  const { data: validatorsData, isLoading } = useGetValidatorsTable(sortBy);

  // first, apply search term
  const searchedData = useMemo(() => {
    return validatorsData?.filter((data) =>
      data.name?.toLowerCase().includes(searchQuery.toLowerCase()),
    );
  }, [validatorsData, searchQuery]);

  // second, sort
  //    Sort by:
  //    Weight
  //    Staked amount
  //    Vote date
  //    Name Asc/Desc
  //    Each vote split

  // third, paginate searched and sorted data
  const startIndex = (currentPage - 1) * itemsPerPage;
  const validators = searchedData?.slice(startIndex, startIndex + itemsPerPage);

  // Sort by:
  // Weight
  // Staked amount
  // Vote date
  // Name Asc/Desc
  // Each vote split

  const totalValidators = searchedData?.length || 0;
  const totalPages = Math.ceil(totalValidators / itemsPerPage);

  const downloadCSV = () => {
    // Implementation for CSV download would go here
    downloadCsvData(validators || []);
  };

  const formatNumber = (num: bigint) => {
    return num.toString();
  };

  const debouncedSearch = useDebounceCallback(setSearchQuery, 500);

  const paginationRange = getPaginationRange(currentPage, totalPages);

  if (isLoading) return <div>Loading...</div>;
  if (!validators) return <div>No data</div>;

  return (
    <div className="w-full text-white">
      <div className="flex flex-col md:flex-row justify-between items-start md:items-center mb-6 gap-4">
        <h1 className="text-2xl font-bold">Top Voters</h1>

        <div className="flex flex-col md:flex-row gap-4 w-full md:w-auto">
          <div className="relative w-full md:w-80">
            <Search
              className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 pointer-events-none"
              size={18}
            />
            <Input
              placeholder="Search..."
              className="w-full pl-10"
              // value={searchQuery}
              onChange={(e) => debouncedSearch(e.target.value)}
            />
          </div>

          <Button onClick={downloadCSV} disabled={isLoading}>
            <Download className="mr-2" size={18} />
            Download CSV
          </Button>

          <div className="flex items-center gap-2">
            <Select value={sortBy} onValueChange={(v: SortBy) => setSortBy(v)}>
              <SelectTrigger className="text-white w-45">
                <div className="flex gap-1">
                  <span className="text-dao-text-secondary">Sort by:</span>
                  <SelectValue placeholder="Weight" />
                </div>
              </SelectTrigger>
              <SelectContent className="text-white">
                {/* // Sort by:
                  // Weight
                  // Staked amount
                  // Vote date
                  // Name Asc/Desc
                  // Each vote split */}
                <SelectItem value="weight">Weight</SelectItem>
                <SelectItem value="name">Name</SelectItem>
                <SelectItem value="percentage">Percentage</SelectItem>
                <SelectItem value="date">Date</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
      </div>

      <div className="overflow-x-auto">
        <Table className="w-full">
          <TableHeader>
            <TableRow className="border-b border-t border-dao-border">
              <TableHead className="text-dao-text-secondary font-normal">
                VALIDATOR
              </TableHead>
              <TableHead className="text-dao-text-secondary font-normal">
                STAKED
              </TableHead>
              <TableHead className="text-dao-text-secondary font-normal">
                VOTER SPLIT
              </TableHead>
              <TableHead className="text-dao-text-secondary font-normal">
                PERCENTAGE
              </TableHead>
              <TableHead className="text-dao-text-secondary font-normal">
                VOTE DATE
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {validators?.map((validator) => (
              <TableRow
                key={validator.vote_identity}
                className="border-b border-dao-border"
              >
                <TableCell className="py-4">
                  <div className="flex items-center gap-3">
                    <div className="p-[2px] w-10 h-10 rounded-lg overflow-hidden bg-gradient-to-b from-[#4A4A4A] to-[#232323]">
                      {validator.image && (
                        // eslint-disable-next-line @next/next/no-img-element
                        <img
                          src={validator.image}
                          alt={validator.name}
                          className="w-full h-full object-cover rounded-md"
                        />
                      )}
                    </div>
                    <div>
                      <div className="font-medium">{validator.name}</div>
                      <div className="text-dao-text-secondary text-sm">
                        {validator.version}
                      </div>
                    </div>
                  </div>
                </TableCell>
                <TableCell>{formatNumber(validator.activated_stake)}</TableCell>
                <TableCell className="px-1">
                  <div className="flex gap-x-6">
                    <div className="flex flex-col">
                      <div className="font-medium">
                        {typeof validator.voterSplits.yes === "number"
                          ? roundDecimals(validator.voterSplits.yes.toString())
                          : validator.voterSplits.yes}
                        %
                      </div>
                      <div className="flex items-center text-sm gap-x-1">
                        <div className="w-2 h-2 rounded-full bg-green-500"></div>
                        <span className="text-dao-text-secondary">Yes</span>
                      </div>
                    </div>
                    <div className="flex flex-col">
                      <div className="font-medium">
                        {typeof validator.voterSplits.no === "number"
                          ? roundDecimals(validator.voterSplits.no.toString())
                          : validator.voterSplits.no}
                        %
                      </div>
                      <div className="flex items-center text-sm gap-x-1">
                        <div className="w-2 h-2 rounded-full bg-red-500"></div>
                        <span className="text-dao-text-secondary">No</span>
                      </div>
                    </div>
                    <div className="flex flex-col">
                      <div className="font-medium">
                        {typeof validator.voterSplits.abstain === "number"
                          ? roundDecimals(
                              validator.voterSplits.abstain.toString(),
                            )
                          : validator.voterSplits.abstain}
                        %
                      </div>
                      <div className="flex items-center text-sm gap-x-1">
                        <div className="w-2 h-2 rounded-full bg-orange-500"></div>
                        <span className="text-dao-text-secondary">Abstain</span>
                      </div>
                    </div>
                    <div className="flex flex-col">
                      <div className="font-medium">
                        {typeof validator.voterSplits.undecided === "number"
                          ? roundDecimals(
                              validator.voterSplits.undecided.toString(),
                            )
                          : validator.voterSplits.undecided}
                        %
                      </div>
                      <div className="flex items-center text-sm gap-x-1">
                        <div className="w-2 h-2 rounded-full bg-gray-500"></div>
                        <span className="text-dao-text-secondary">
                          Undecided
                        </span>
                      </div>
                    </div>
                  </div>
                </TableCell>
                <TableCell className="">{validator.percentage}%</TableCell>
                <TableCell className="">{validator.voteDate}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      <div className="flex flex-col md:flex-row justify-between items-center mt-6 gap-4">
        <div className="text-white">Total Validators: {totalValidators}</div>

        <div className="flex items-center gap-2">
          <Button
            size="icon"
            className="bg-gray-900 border-gray-700 text-white hover:bg-gray-800"
            onClick={() => setCurrentPage(Math.max(1, currentPage - 1))}
            disabled={currentPage === 1}
          >
            <ChevronLeft size={16} />
          </Button>

          {paginationRange.map((page) => (
            <Button
              key={page}
              variant={currentPage === page ? "outline" : "default"}
              // className={currentPage === page ? "" : "text-dao-text-secondary"}
              className="w-9 px-3.5"
              onClick={
                typeof page === "string"
                  ? undefined
                  : () => setCurrentPage(page)
              }
              disabled={typeof page === "string"}
            >
              {page}
            </Button>
          ))}

          <Button
            size="icon"
            className="bg-gray-900 border-gray-700 text-white hover:bg-gray-800"
            onClick={() =>
              setCurrentPage(Math.min(totalPages, currentPage + 1))
            }
            disabled={currentPage === totalPages}
          >
            <ChevronRight size={16} />
          </Button>
        </div>

        <div className="flex items-center gap-2">
          <span className="text-white">Show Per Page:</span>
          <Select
            value={itemsPerPage.toString()}
            onValueChange={(value) => setItemsPerPage(Number.parseInt(value))}
          >
            <SelectTrigger className="bg-gray-900 border-gray-700 text-white w-16">
              <SelectValue placeholder="5" />
            </SelectTrigger>
            <SelectContent className="bg-gray-900 border-gray-700 text-white">
              <SelectItem value="5">5</SelectItem>
              <SelectItem value="10">10</SelectItem>
              <SelectItem value="20">20</SelectItem>
              <SelectItem value="50">50</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>
    </div>
  );
}

const getPaginationRange = (currentPage: number, totalPages: number) => {
  const delta = 2; // how many pages to show around the current page
  const range = [];

  const left = Math.max(2, currentPage - delta);
  const right = Math.min(totalPages - 1, currentPage + delta);

  range.push(1);
  if (left > 2) range.push("...");

  for (let i = left; i <= right; i++) {
    range.push(i);
  }

  if (right < totalPages - 1) range.push("...");
  if (totalPages > 1) range.push(totalPages);

  return range;
};
