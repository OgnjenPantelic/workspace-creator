import React from 'react';
import { CLOUDS } from '../../constants';

interface CloudSelectionScreenProps {
  loadingCloud: string | null;
  onSelectCloud: (cloud: string) => void;
  onBack: () => void;
}

const CloudSelectionScreen: React.FC<CloudSelectionScreenProps> = ({
  loadingCloud,
  onSelectCloud,
  onBack,
}) => {
  return (
    <div className="container">
      <button className="back-btn" onClick={onBack}>
        ← Back
      </button>
      <h1>Select Cloud Provider</h1>
      <p className="subtitle">Choose where you want to deploy your Databricks workspace.</p>
      <div className="cloud-options">
        <button
          className={`cloud-option aws ${loadingCloud === CLOUDS.AWS ? "loading" : ""}`}
          onClick={() => onSelectCloud(CLOUDS.AWS)}
          disabled={loadingCloud !== null}
        >
          {loadingCloud === CLOUDS.AWS ? (
            <>
              <span className="spinner" />
              Loading...
            </>
          ) : (
            <>
              <span className="cloud-logo aws-logo">AWS</span>
              <span className="cloud-name">Amazon Web Services</span>
            </>
          )}
        </button>
        <button
          className={`cloud-option azure ${loadingCloud === CLOUDS.AZURE ? "loading" : ""}`}
          onClick={() => onSelectCloud(CLOUDS.AZURE)}
          disabled={loadingCloud !== null}
        >
          {loadingCloud === CLOUDS.AZURE ? (
            <>
              <span className="spinner" />
              Loading...
            </>
          ) : (
            <>
              <span className="cloud-logo azure-logo">Azure</span>
              <span className="cloud-name">Microsoft Azure</span>
            </>
          )}
        </button>
        <button
          className="cloud-option gcp disabled"
          disabled={true}
          title="GCP support coming soon"
        >
          <span className="cloud-logo gcp-logo">GCP</span>
          <span className="cloud-name">Google Cloud (Coming Soon)</span>
        </button>
      </div>
    </div>
  );
};

export default CloudSelectionScreen;
