import { useMDXComponents as getDocsMDXComponents } from 'nextra-theme-docs';

const docsComponents = getDocsMDXComponents();

export const useMDXComponents = (components) => ({
  ...docsComponents,
  ...components,
});

// RepoLink component removed - use regular markdown links instead
// Example: [link text](https://github.com/3uild-3thos/govcontract/blob/main/path/to/file.rs)

// ArgTable component for displaying command arguments
export const ArgTable = ({ children }) => {
  return (
    <div style={{ overflowX: 'auto', margin: '1rem 0' }}>
      <table style={{ width: '100%', borderCollapse: 'collapse' }}>
        <thead>
          <tr style={{ borderBottom: '2px solid #e5e7eb' }}>
            <th style={{ padding: '0.5rem', textAlign: 'left', fontWeight: 'bold' }}>Name</th>
            <th style={{ padding: '0.5rem', textAlign: 'left', fontWeight: 'bold' }}>Type</th>
            <th style={{ padding: '0.5rem', textAlign: 'left', fontWeight: 'bold' }}>Required</th>
            <th style={{ padding: '0.5rem', textAlign: 'left', fontWeight: 'bold' }}>Default</th>
            <th style={{ padding: '0.5rem', textAlign: 'left', fontWeight: 'bold' }}>Description</th>
          </tr>
        </thead>
        <tbody>
          {children}
        </tbody>
      </table>
    </div>
  );
};

// ArgRow component for table rows
export const ArgRow = ({ name, type, required, defaultValue, description }) => {
  return (
    <tr style={{ borderBottom: '1px solid #e5e7eb' }}>
      <td style={{ padding: '0.5rem', fontFamily: 'monospace', fontWeight: 'bold' }}>{name}</td>
      <td style={{ padding: '0.5rem', fontFamily: 'monospace' }}>{type}</td>
      <td style={{ padding: '0.5rem' }}>{required ? 'Yes' : 'No'}</td>
      <td style={{ padding: '0.5rem', fontFamily: 'monospace' }}>{defaultValue || '-'}</td>
      <td style={{ padding: '0.5rem' }}>{description}</td>
    </tr>
  );
};
