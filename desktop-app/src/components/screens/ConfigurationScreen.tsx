import { useMemo, useState } from "react";
import { 
  CLOUDS, 
  AWS_REGIONS, 
  AZURE_REGIONS, 
  GCP_REGIONS, 
  EXCLUDE_VARIABLES,
  VARIABLE_DESCRIPTION_OVERRIDES 
} from "../../constants";
import { groupVariablesBySection, formatVariableName } from "../../utils/variables";
import { useWizard } from "../../hooks/useWizard";

export function ConfigurationScreen() {
  const {
    selectedTemplate, selectedCloud,
    variables, formValues, setFormValues,
    tagPairs, setTagPairs,
    loading, error,
    showAdvanced, setShowAdvanced,
    formSubmitAttempted, setFormSubmitAttempted,
    setScreen, goBack,
    azure,
  } = useWizard();
  const azureResourceGroups = azure.resourceGroups;
  const [showTags, setShowTags] = useState(false);
  const [createNewVpc, setCreateNewVpc] = useState(true);
  const onContinue = () => setScreen("unity-catalog-config");
  const onBack = goBack;
  // Form handlers
  const handleFormChange = (name: string, value: any) => {
    setFormValues((prev) => {
      const updated = { ...prev, [name]: value };
      // Auto-fill vnet_resource_group_name when resource_group_name changes
      if (name === "resource_group_name" && selectedCloud === CLOUDS.AZURE) {
        updated["vnet_resource_group_name"] = value;
        // Set create_new_resource_group based on whether this is an existing RG
        const isExistingRg = azureResourceGroups.some(rg => rg.name === value);
        updated["create_new_resource_group"] = !isExistingRg;
      }
      // When toggling create_new_vnet to true, sync vnet_resource_group_name from resource_group_name
      if (name === "create_new_vnet" && value === true && selectedCloud === CLOUDS.AZURE) {
        updated["vnet_resource_group_name"] = prev["resource_group_name"] || "";
      }
      return updated;
    });
  };

  const updateTagsFormValue = (pairs: { key: string; value: string }[]) => {
    const validPairs = pairs.filter(p => p.key.trim() !== "");
    if (validPairs.length === 0) {
      setFormValues(prev => ({ ...prev, tags: "" }));
    } else {
      const tagObj: Record<string, string> = {};
      validPairs.forEach(p => { tagObj[p.key] = p.value; });
      setFormValues(prev => ({ ...prev, tags: JSON.stringify(tagObj) }));
    }
  };

  const handleTagChange = (index: number, field: "key" | "value", value: string) => {
    setTagPairs(prev => {
      const updated = [...prev];
      updated[index] = { ...updated[index], [field]: value };
      updateTagsFormValue(updated);
      return updated;
    });
  };

  const addTag = () => {
    setTagPairs(prev => [...prev, { key: "", value: "" }]);
  };

  const removeTag = (index: number) => {
    setTagPairs(prev => {
      const updated = prev.filter((_, i) => i !== index);
      updateTagsFormValue(updated);
      return updated;
    });
  };

  // Determine which fields are hidden by toggles (should not be validated)
  const createNewVnetValue = formValues.create_new_vnet;
  const createNewVnet = createNewVnetValue === true || createNewVnetValue === "true" || createNewVnetValue === undefined;
  const hiddenFields = new Set<string>();
  // Azure: hidden fields based on VNet toggle
  if (createNewVnet) {
    hiddenFields.add("vnet_name");  // Only hide vnet_name for new VNet
  } else {
    hiddenFields.add("cidr");
  }
  // AWS: hidden fields based on VPC toggle
  if (createNewVpc) {
    hiddenFields.add("existing_vpc_id");
    hiddenFields.add("existing_subnet_ids");
    hiddenFields.add("existing_security_group_id");
  } else {
    hiddenFields.add("cidr_block");
  }

  // Fields that are always required in the UI, even if they have Terraform defaults
  const alwaysRequired = new Set<string>([
    "prefix", "workspace_name", "databricks_workspace_name", "admin_user",
  ]);

  // Conditionally required fields based on toggle state
  const conditionallyRequired = new Set<string>();
  if (selectedCloud === CLOUDS.AWS) {
    if (createNewVpc) {
      conditionallyRequired.add("cidr_block");
    } else {
      conditionallyRequired.add("existing_vpc_id");
      conditionallyRequired.add("existing_subnet_ids");
      conditionallyRequired.add("existing_security_group_id");
    }
  }
  if (selectedCloud === CLOUDS.AZURE) {
    // vnet_resource_group_name is always required (needed for both new and existing)
    conditionallyRequired.add("vnet_resource_group_name");
    
    if (createNewVnet) {
      conditionallyRequired.add("cidr");
      conditionallyRequired.add("subnet_public_cidr");
      conditionallyRequired.add("subnet_private_cidr");
    } else {
      conditionallyRequired.add("vnet_name");
      conditionallyRequired.add("subnet_public_cidr");
      conditionallyRequired.add("subnet_private_cidr");
    }
  }
  if (selectedCloud === CLOUDS.GCP) {
    conditionallyRequired.add("subnet_cidr");
  }

  // Combined set of all required field names (for label rendering)
  const allRequiredFields = new Set<string>([
    ...alwaysRequired,
    ...conditionallyRequired,
  ]);

  // Memoized validation for required form fields
  const formValidation = useMemo(() => {
    // Terraform-required (no default) + always-required (have defaults but must be filled)
    const requiredVars = variables.filter(v => 
      !EXCLUDE_VARIABLES.includes(v.name as any) &&
      !hiddenFields.has(v.name) &&
      ((v.required && !v.default) || alwaysRequired.has(v.name))
    );
    
    const missingFields = requiredVars.filter(v => {
      const value = formValues[v.name];
      return value === undefined || value === null || value === "";
    });

    // Add conditionally required fields that are empty
    const conditionalMissing = [...conditionallyRequired].filter(name => {
      const value = formValues[name];
      return value === undefined || value === null || value === "" || 
        (Array.isArray(value) && value.length === 0);
    });
    
    const fieldErrors: Record<string, string> = {};
    const prefixValue = formValues["prefix"];
    if (prefixValue) {
      if (!/^[a-z0-9][a-z0-9-]*[a-z0-9]$|^[a-z0-9]$/.test(prefixValue)) {
        fieldErrors["prefix"] = "Must start and end with a letter or number, contain only lowercase letters, numbers, and hyphens";
      } else if (prefixValue.length < 3 || prefixValue.length > 30) {
        fieldErrors["prefix"] = "Must be between 3 and 30 characters";
      } else if (/--/.test(prefixValue)) {
        fieldErrors["prefix"] = "Cannot contain consecutive hyphens";
      }
    }
    
    const gcpWorkspaceName = formValues["databricks_workspace_name"];
    if (gcpWorkspaceName) {
      if (!/^[a-z0-9][a-z0-9-]*[a-z0-9]$|^[a-z0-9]$/.test(gcpWorkspaceName)) {
        fieldErrors["databricks_workspace_name"] = "Must start and end with a letter or number, contain only lowercase letters, numbers, and hyphens";
      } else if (gcpWorkspaceName.length < 3 || gcpWorkspaceName.length > 30) {
        fieldErrors["databricks_workspace_name"] = "Must be between 3 and 30 characters";
      } else if (/--/.test(gcpWorkspaceName)) {
        fieldErrors["databricks_workspace_name"] = "Cannot contain consecutive hyphens";
      }
    }

    const allMissing = [
      ...missingFields.map(v => v.name),
      ...conditionalMissing,
    ];
    
    return {
      isValid: allMissing.length === 0 && Object.keys(fieldErrors).length === 0,
      missingFields: allMissing,
      requiredFields: [...requiredVars.map(v => v.name), ...conditionallyRequired],
      fieldErrors,
    };
  }, [variables, formValues, hiddenFields, conditionallyRequired]);

  if (!selectedTemplate || loading) {
    return (
      <div className="container">
        <div className="loading">Loading configuration...</div>
      </div>
    );
  }

  const sections = groupVariablesBySection(variables);

  return (
    <div className="container">
      <button className="back-btn" onClick={onBack}>
        ← Back
      </button>
      <h1>Configure Deployment</h1>
      <p className="subtitle">
        Fill in the configuration values for your Databricks workspace deployment.
      </p>

      {error && <div className="alert alert-error">{error}</div>}

      <form onSubmit={(e) => e.preventDefault()}>
        {/* Variable Sections */}
        {Object.entries(sections)
          .filter(([sectionName]) => !sectionName.startsWith("Advanced") && sectionName !== "Tags")
          .map(([sectionName, sectionVars]) => (
          <div key={sectionName} className={`form-section ${selectedCloud}`}>
            <h3>{sectionName}</h3>
            <div className="two-column">
              {sectionVars
                .filter((variable) => {
                  const createNewVnetValue = formValues.create_new_vnet;
                  const createNewVnet = createNewVnetValue === true || createNewVnetValue === "true" || createNewVnetValue === undefined;
                  if (createNewVnet && variable.name === "vnet_name") return false;
                  if (!createNewVnet && variable.name === "cidr") return false;
                  return true;
                })
                .map((variable) => (
                <div key={variable.name} className="form-group" style={variable.name === "tags" ? { gridColumn: "1 / -1" } : undefined}>
                  <label>
                    {formatVariableName(variable.name)}
                    {((variable.required && !variable.default) || allRequiredFields.has(variable.name)) && " *"}
                  </label>
                  {variable.var_type.includes("bool") ? (
                    <label className="checkbox-label">
                      <input
                        type="checkbox"
                        checked={formValues[variable.name] === "true" || formValues[variable.name] === true}
                        onChange={(e) => handleFormChange(variable.name, e.target.checked)}
                      />
                      Enable
                    </label>
                  ) : variable.name === "workspace_sku" ? (
                    <select
                      value={formValues[variable.name] || variable.default || "premium"}
                      onChange={(e) => handleFormChange(variable.name, e.target.value)}
                    >
                      <option value="standard">Standard</option>
                      <option value="premium">Premium</option>
                      <option value="trial">Trial</option>
                    </select>
                  ) : variable.name === "region" ? (
                    <select
                      value={formValues[variable.name] || variable.default || "us-east-1"}
                      onChange={(e) => handleFormChange(variable.name, e.target.value)}
                    >
                      {AWS_REGIONS.map((region) => (
                        <option key={region.value} value={region.value}>
                          {region.label}
                        </option>
                      ))}
                    </select>
                  ) : variable.name === "location" ? (
                    <select
                      value={formValues[variable.name] || variable.default || "eastus2"}
                      onChange={(e) => handleFormChange(variable.name, e.target.value)}
                    >
                      {AZURE_REGIONS.map((region) => (
                        <option key={region.value} value={region.value}>
                          {region.label}
                        </option>
                      ))}
                    </select>
                  ) : variable.name === "google_region" ? (
                    <select
                      value={formValues[variable.name] || variable.default || "us-central1"}
                      onChange={(e) => handleFormChange(variable.name, e.target.value)}
                    >
                      {GCP_REGIONS.map((region) => (
                        <option key={region.value} value={region.value}>
                          {region.label}
                        </option>
                      ))}
                    </select>
                  ) : variable.name === "resource_group_name" && azureResourceGroups.length > 0 ? (
                    <>
                      <select
                        value={azureResourceGroups.some(rg => rg.name === formValues[variable.name]) ? formValues[variable.name] : ""}
                        onChange={(e) => {
                          const val = e.target.value;
                          setFormValues(prev => ({
                            ...prev,
                            [variable.name]: val,
                            vnet_resource_group_name: val,
                            create_new_resource_group: val === "" ? true : false,
                          }));
                        }}
                        className={formSubmitAttempted && formValidation.missingFields.includes(variable.name) ? "input-error" : ""}
                      >
                        <option value="">Select existing or create new below</option>
                        {azureResourceGroups.map((rg) => (
                          <option key={rg.name} value={rg.name}>
                            {rg.name} ({rg.location})
                          </option>
                        ))}
                      </select>
                      <input
                        type="text"
                        autoCapitalize="none"
                        autoCorrect="off"
                        spellCheck={false}
                        value={azureResourceGroups.some(rg => rg.name === formValues[variable.name]) ? "" : (formValues[variable.name] || "")}
                        onChange={(e) => {
                          const val = e.target.value;
                          const isExisting = azureResourceGroups.some(
                            rg => rg.name.toLowerCase() === val.toLowerCase()
                          );
                          const actualName = isExisting 
                            ? azureResourceGroups.find(rg => rg.name.toLowerCase() === val.toLowerCase())?.name || val
                            : val;
                          setFormValues(prev => ({
                            ...prev,
                            [variable.name]: actualName,
                            vnet_resource_group_name: actualName,
                            create_new_resource_group: !isExisting,
                          }));
                        }}
                        placeholder="Or enter new resource group name"
                        style={{ marginTop: "8px" }}
                        className={formSubmitAttempted && formValidation.missingFields.includes(variable.name) ? "input-error" : ""}
                      />
                    </>
                  ) : variable.name === "tags" ? (
                    <div className="tags-editor">
                      {tagPairs.map((tag, index) => (
                        <div key={index} style={{ display: "flex", gap: "8px", marginBottom: "8px", alignItems: "center" }}>
                          <input
                            type="text"
                            autoCapitalize="none"
                            autoCorrect="off"
                            spellCheck={false}
                            value={tag.key}
                            onChange={(e) => handleTagChange(index, "key", e.target.value)}
                            placeholder="Key (e.g., Environment)"
                            style={{ flex: 1 }}
                          />
                          <input
                            type="text"
                            autoCapitalize="none"
                            autoCorrect="off"
                            spellCheck={false}
                            value={tag.value}
                            onChange={(e) => handleTagChange(index, "value", e.target.value)}
                            placeholder="Value (e.g., Production)"
                            style={{ flex: 1 }}
                          />
                          <button
                            type="button"
                            onClick={() => removeTag(index)}
                            style={{
                              background: "transparent",
                              border: "1px solid #555",
                              color: "#e74c3c",
                              borderRadius: "4px",
                              padding: "6px 10px",
                              cursor: "pointer",
                              fontSize: "14px"
                            }}
                            title="Remove tag"
                          >
                            ×
                          </button>
                        </div>
                      ))}
                      <button
                        type="button"
                        onClick={addTag}
                        style={{
                          background: "transparent",
                          border: "1px dashed #555",
                          color: "#888",
                          borderRadius: "4px",
                          padding: "8px 16px",
                          cursor: "pointer",
                          fontSize: "13px",
                          width: "100%"
                        }}
                      >
                        + Add Tag
                      </button>
                    </div>
                  ) : (
                    <input
                      type={variable.sensitive ? "password" : "text"}
                      autoCapitalize="none"
                      autoCorrect="off"
                      spellCheck={false}
                      value={formValues[variable.name] || ""}
                      onChange={(e) => handleFormChange(variable.name, e.target.value)}
                      placeholder={variable.default || ""}
                      className={
                        (formSubmitAttempted && formValidation.missingFields.includes(variable.name)) ||
                        (formValidation.fieldErrors[variable.name]) ? "input-error" : ""
                      }
                    />
                  )}
                  {formValidation.fieldErrors[variable.name] && (
                    <div className="help-text" style={{ color: "#e74c3c" }}>
                      {formValidation.fieldErrors[variable.name]}
                    </div>
                  )}
                  {(VARIABLE_DESCRIPTION_OVERRIDES[variable.name] || variable.description) && !formValidation.fieldErrors[variable.name] && (
                    <div className="help-text">
                      {VARIABLE_DESCRIPTION_OVERRIDES[variable.name] || variable.description}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        ))}

        {/* Advanced: Network Configuration (collapsible) */}
        {Object.entries(sections)
          .filter(([sectionName]) => sectionName.startsWith("Advanced"))
          .map(([sectionName, sectionVars]) => (
          <div key={sectionName} className={`form-section advanced ${showAdvanced ? "expanded" : ""}`}>
            <h3 
              onClick={() => setShowAdvanced(!showAdvanced)} 
              style={{ cursor: "pointer", display: "flex", alignItems: "center", gap: "8px" }}
            >
              <span style={{ fontSize: "12px" }}>{showAdvanced ? "▼" : "►"}</span>
              Advanced: Network Configuration
            </h3>
            {showAdvanced && (
              <>
                <p style={{ color: "#888", marginBottom: "16px", fontSize: "0.85em" }}>
                  Network settings have sensible defaults. Modify only if you have specific requirements.
                </p>
                {/* AWS: Create New VPC toggle */}
                {selectedCloud === CLOUDS.AWS && (
                  <div className="form-group" style={{ marginBottom: "16px" }}>
                    <label className="checkbox-label">
                      <input
                        type="checkbox"
                        checked={createNewVpc}
                        onChange={(e) => {
                          setCreateNewVpc(e.target.checked);
                          if (e.target.checked) {
                            // Clear existing VPC fields when switching back to new
                            setFormValues(prev => ({
                              ...prev,
                              existing_vpc_id: "",
                              existing_subnet_ids: "",
                              existing_security_group_id: "",
                            }));
                          }
                        }}
                      />
                      Create New VPC
                    </label>
                    <div className="help-text">
                      {createNewVpc 
                        ? "A new VPC will be created with the CIDR block below."
                        : "Use an existing VPC by providing its ID, subnet IDs, and security group ID."}
                    </div>
                  </div>
                )}
                <div className="two-column">
                  {sectionVars
                    .filter((variable) => {
                      // Azure: toggle between new VNet and existing VNet fields
                      const createNewVnetValue = formValues.create_new_vnet;
                      const createNewVnet = createNewVnetValue === true || createNewVnetValue === "true" || createNewVnetValue === undefined;
                      if (createNewVnet && variable.name === "vnet_name") return false;
                      if (!createNewVnet && variable.name === "cidr") return false;
                      // AWS: toggle between new VPC and existing VPC fields
                      if (createNewVpc && variable.name === "existing_vpc_id") return false;
                      if (createNewVpc && variable.name === "existing_subnet_ids") return false;
                      if (createNewVpc && variable.name === "existing_security_group_id") return false;
                      if (!createNewVpc && variable.name === "cidr_block") return false;
                      return true;
                    })
                    .map((variable) => (
                    <div key={variable.name} className="form-group">
                      <label>
                        {formatVariableName(variable.name)}
                        {((variable.required && !variable.default) || allRequiredFields.has(variable.name)) && " *"}
                      </label>
                      {variable.var_type.includes("bool") ? (
                        <label className="checkbox-label">
                          <input
                            type="checkbox"
                            checked={formValues[variable.name] === "true" || formValues[variable.name] === true}
                            onChange={(e) => handleFormChange(variable.name, e.target.checked)}
                          />
                          Enable
                        </label>
                      ) : (
                        <input
                          type={variable.sensitive ? "password" : "text"}
                          autoCapitalize="none"
                          autoCorrect="off"
                          spellCheck={false}
                          value={formValues[variable.name] || ""}
                          onChange={(e) => handleFormChange(variable.name, e.target.value)}
                          placeholder={variable.default || ""}
                          className={
                            (formSubmitAttempted && formValidation.missingFields.includes(variable.name)) ||
                            (formValidation.fieldErrors[variable.name]) ? "input-error" : ""
                          }
                        />
                      )}
                      {formValidation.fieldErrors[variable.name] && (
                        <div className="help-text" style={{ color: "#e74c3c" }}>
                          {formValidation.fieldErrors[variable.name]}
                        </div>
                      )}
                      {(VARIABLE_DESCRIPTION_OVERRIDES[variable.name] || variable.description) && !formValidation.fieldErrors[variable.name] && (
                        <div className="help-text">
                          {VARIABLE_DESCRIPTION_OVERRIDES[variable.name] || variable.description}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </>
            )}
          </div>
        ))}

        {/* Tags (collapsible) */}
        {sections["Tags"] && (
          <div className={`form-section advanced ${showTags ? "expanded" : ""}`}>
            <h3
              onClick={() => setShowTags(!showTags)}
              style={{ cursor: "pointer", display: "flex", alignItems: "center", gap: "8px" }}
            >
              <span style={{ fontSize: "12px" }}>{showTags ? "▼" : "►"}</span>
              Tags
            </h3>
            {showTags && (
              <>
                <p style={{ color: "#888", marginBottom: "16px", fontSize: "0.85em" }}>
                  Optional key-value pairs to tag all created resources for cost tracking and organization.
                </p>
                <div className="tags-editor">
                  {tagPairs.map((tag, index) => (
                    <div key={index} style={{ display: "flex", gap: "8px", marginBottom: "8px", alignItems: "center" }}>
                      <input
                        type="text"
                        autoCapitalize="none"
                        autoCorrect="off"
                        spellCheck={false}
                        value={tag.key}
                        onChange={(e) => handleTagChange(index, "key", e.target.value)}
                        placeholder="Key (e.g., Environment)"
                        style={{ flex: 1 }}
                      />
                      <input
                        type="text"
                        autoCapitalize="none"
                        autoCorrect="off"
                        spellCheck={false}
                        value={tag.value}
                        onChange={(e) => handleTagChange(index, "value", e.target.value)}
                        placeholder="Value (e.g., Production)"
                        style={{ flex: 1 }}
                      />
                      <button
                        type="button"
                        onClick={() => removeTag(index)}
                        style={{
                          background: "transparent",
                          border: "1px solid #555",
                          color: "#e74c3c",
                          borderRadius: "4px",
                          padding: "6px 10px",
                          cursor: "pointer",
                          fontSize: "14px"
                        }}
                        title="Remove tag"
                      >
                        ×
                      </button>
                    </div>
                  ))}
                  <button
                    type="button"
                    onClick={addTag}
                    style={{
                      background: "transparent",
                      border: "1px dashed #555",
                      color: "#888",
                      borderRadius: "4px",
                      padding: "8px 16px",
                      cursor: "pointer",
                      fontSize: "13px",
                      width: "100%"
                    }}
                  >
                    + Add Tag
                  </button>
                </div>
              </>
            )}
          </div>
        )}

        <div style={{ marginTop: "32px" }}>
          <button 
            className="btn btn-large btn-success" 
            onClick={() => {
              if (!formValidation.isValid) {
                setFormSubmitAttempted(true);
              } else {
                onContinue();
              }
            }} 
            disabled={loading}
          >
            {loading ? (
              <>
                <span className="spinner" />
                Preparing...
              </>
            ) : (
              "Continue →"
            )}
          </button>
          {formSubmitAttempted && !formValidation.isValid && (
            <p style={{ marginTop: "12px", color: "#e74c3c", fontSize: "0.9em" }}>
              {formValidation.missingFields.length > 0 && (
                <>Please fill in all required fields: {formValidation.missingFields.map(f => formatVariableName(f)).join(", ")}</>
              )}
              {formValidation.missingFields.length > 0 && Object.keys(formValidation.fieldErrors).length > 0 && <br />}
              {Object.keys(formValidation.fieldErrors).length > 0 && (
                <>Please fix validation errors: {Object.keys(formValidation.fieldErrors).map(f => formatVariableName(f)).join(", ")}</>
              )}
            </p>
          )}
          {formValidation.isValid && (
            <p style={{ marginTop: "12px", color: "#888", fontSize: "0.9em" }}>
              This will validate your configuration and guide you through the deployment process.
            </p>
          )}
        </div>
      </form>
    </div>
  );
}
