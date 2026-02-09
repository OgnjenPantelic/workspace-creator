import React from 'react';
import { useWizard } from '../../hooks/useWizard';

const WelcomeScreen: React.FC = () => {
  const { setScreen, deployment } = useWizard();

  return (
    <div className="container">
      <div className="welcome-content">
        <h1 className="gradient" style={{ fontSize: "3em", marginBottom: "20px" }}>
          Databricks Deployer
        </h1>
        <p className="subtitle">
          Deploy Databricks workspaces with ease
        </p>

        <div className="welcome-intro">
          <p>
            Setting up Databricks workspaces with proper networking, security, and 
            Unity Catalog can be complex.{" "}
            <strong style={{ color: "#ff6b35" }}>This tool simplifies deployment</strong>{" "}
            using proven Terraform templates that follow Databricks best practices.
          </p>
          <p>
            No Terraform experience required. Follow the guided steps, configure your 
            options, and deploy a production-ready workspace.
          </p>
        </div>

        <div style={{ marginTop: "50px" }}>
          <button className="btn btn-large" onClick={() => setScreen("cloud-selection")}>
            Get Started →
          </button>
        </div>

        <div className="feature-grid">
          <div className="feature-item">
            <div className="feature-icon">🚀</div>
            <div className="feature-title">Fast Deployment</div>
            <div className="feature-description">Deploy in minutes, not days</div>
          </div>
          <div className="feature-item">
            <div className="feature-icon">🔒</div>
            <div className="feature-title">Enterprise Security</div>
            <div className="feature-description">Best practices built-in</div>
          </div>
          <div className="feature-item">
            <div className="feature-icon">✨</div>
            <div className="feature-title">No Code Required</div>
            <div className="feature-description">Simple, guided experience</div>
          </div>
        </div>

        <div style={{ marginTop: "40px" }}>
          <button 
            onClick={deployment.openDeploymentsFolder}
            style={{
              background: "transparent",
              border: "none",
              color: "#666",
              cursor: "pointer",
              fontSize: "13px",
              textDecoration: "underline"
            }}
          >
            View previous deployments folder
          </button>
        </div>
      </div>
    </div>
  );
};

export default WelcomeScreen;
