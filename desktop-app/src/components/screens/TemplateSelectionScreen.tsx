import { CLOUDS } from "../../constants";
import { useWizard } from "../../hooks/useWizard";

const CheckIcon = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--success)" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" style={{ flexShrink: 0 }}>
    <polyline points="20 6 9 17 4 12" />
  </svg>
);

export function TemplateSelectionScreen() {
  const { templates, selectedCloud, selectTemplate, goBack } = useWizard();
  const cloudTemplates = templates.filter((t) => t.cloud === selectedCloud);

  const handleKeyDown = (e: React.KeyboardEvent, template: typeof cloudTemplates[0]) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      selectTemplate(template);
    }
  };

  return (
    <div className="container">
      <button className="back-btn" onClick={goBack}>
        ← Back
      </button>
      <h1>Select Template</h1>
      <p className="subtitle">
        Choose the security and networking configuration that best fits your requirements.
      </p>

      <div className="templates">
        {cloudTemplates.length === 0 ? (
          <div className="template-card" style={{ opacity: 0.6, cursor: "not-allowed" }}>
            <div className="coming-soon">Coming Soon</div>
            <div className="template-title">
              {selectedCloud === CLOUDS.GCP ? "GCP Standard Workspace" : "Standard Workspace"}
            </div>
            <div className="template-description">
              Templates for this cloud provider are not yet available.
            </div>
          </div>
        ) : (
          <>
            {cloudTemplates.map((template) => (
              <div
                key={template.id}
                className="template-card"
                onClick={() => selectTemplate(template)}
                onKeyDown={(e) => handleKeyDown(e, template)}
                tabIndex={0}
                role="button"
                aria-label={`Select ${template.name} template`}
              >
                <div className="template-title">{template.name}</div>
                <div className="template-description">{template.description}</div>
                <div className="template-features">
                  <ul>
                    {template.features.map((feature, i) => (
                      <li key={i} style={{ display: "flex", alignItems: "flex-start", gap: "8px" }}>
                        <CheckIcon />
                        <span>{feature}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}
