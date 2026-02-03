import { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  DependencyStatus,
  Template,
  TerraformVariable,
  DeploymentStatus,
  CloudCredentials,
  AppScreen,
  AwsProfile,
  AwsIdentity,
  AzureSubscription,
  AzureAccount,
} from "./types";
import {
  CLOUDS,
  POLLING,
  CLOUD_DISPLAY_NAMES,
  VARIABLE_DISPLAY_NAMES,
  VARIABLE_DESCRIPTION_OVERRIDES,
  EXCLUDE_VARIABLES,
  AWS_REGIONS,
  AZURE_REGIONS,
} from "./constants";

// Deployment wizard steps
type DeploymentStep = "ready" | "initializing" | "planning" | "review" | "deploying" | "complete" | "failed";

// Generate random suffix for resource names
const generateRandomSuffix = () => {
  return Math.random().toString(36).substring(2, 8);
};

function App() {
  const [screen, setScreen] = useState<AppScreen>("welcome");
  const [dependencies, setDependencies] = useState<Record<string, DependencyStatus>>({});
  const [templates, setTemplates] = useState<Template[]>([]);
  const [selectedCloud, setSelectedCloud] = useState<string>("");
  const [selectedTemplate, setSelectedTemplate] = useState<Template | null>(null);
  const [variables, setVariables] = useState<TerraformVariable[]>([]);
  const [formValues, setFormValues] = useState<Record<string, any>>({});
  const [credentials, setCredentials] = useState<CloudCredentials>({});
  const [deploymentStatus, setDeploymentStatus] = useState<DeploymentStatus | null>(null);
  const [showDetailedLogs, setShowDetailedLogs] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [installingTerraform, setInstallingTerraform] = useState(false);
  const [deploymentStep, setDeploymentStep] = useState<DeploymentStep>("ready");
  const [loadingCloud, setLoadingCloud] = useState<string | null>(null);
  const [validatingCredentials, setValidatingCredentials] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [formSubmitAttempted, setFormSubmitAttempted] = useState(false);
  const [tagPairs, setTagPairs] = useState<{key: string, value: string}[]>([]);
  const [awsProfiles, setAwsProfiles] = useState<AwsProfile[]>([]);
  const [awsIdentity, setAwsIdentity] = useState<AwsIdentity | null>(null);
  const [awsAuthMode, setAwsAuthMode] = useState<"profile" | "keys">("profile");
  const [awsAuthError, setAwsAuthError] = useState<string | null>(null);
  const [awsLoading, setAwsLoading] = useState(false);
  
  // Azure credential states
  const [azureAccount, setAzureAccount] = useState<AzureAccount | null>(null);
  const [azureSubscriptions, setAzureSubscriptions] = useState<AzureSubscription[]>([]);
  const [azureResourceGroups, setAzureResourceGroups] = useState<{name: string, location: string}[]>([]);
  const [azureResourceGroupsCacheKey, setAzureResourceGroupsCacheKey] = useState<string>("");
  const [azureAuthMode, setAzureAuthMode] = useState<"cli" | "service_principal">("cli");
  const [azureAuthError, setAzureAuthError] = useState<string | null>(null);
  const [azureLoading, setAzureLoading] = useState(false);
  
  // Deployment tracking states
  const [isRollingBack, setIsRollingBack] = useState(false);
  const [templatePath, setTemplatePath] = useState<string>("");
  const [deploymentName, setDeploymentName] = useState<string>("");
  
  // Refs to track current state in async callbacks
  const deploymentStepRef = useRef<DeploymentStep>("ready");
  const selectedTemplateRef = useRef<Template | null>(null);
  const credentialsRef = useRef<CloudCredentials>({});
  const deploymentNameRef = useRef<string>("");
  
  // Refs for interval cleanup (prevent memory leaks)
  const pollIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const ssoPollingRef = useRef<ReturnType<typeof setInterval> | null>(null);
  
  // Track if component is mounted to prevent state updates after unmount
  const isMountedRef = useRef(true);
  
  // Helper to clear SSO polling interval
  const clearSsoPolling = () => {
    if (ssoPollingRef.current) {
      clearInterval(ssoPollingRef.current);
      ssoPollingRef.current = null;
    }
  };
  
  // Helper to clear deployment polling interval
  const clearPollInterval = () => {
    if (pollIntervalRef.current) {
      clearInterval(pollIntervalRef.current);
      pollIntervalRef.current = null;
    }
  };
  
  // Keep refs in sync with state
  useEffect(() => {
    deploymentStepRef.current = deploymentStep;
  }, [deploymentStep]);
  
  useEffect(() => {
    selectedTemplateRef.current = selectedTemplate;
  }, [selectedTemplate]);
  
  useEffect(() => {
    credentialsRef.current = credentials;
  }, [credentials]);
  
  useEffect(() => {
    deploymentNameRef.current = deploymentName;
  }, [deploymentName]);

  // Load templates on mount
  useEffect(() => {
    loadTemplates();
  }, []);

  // Cleanup intervals on unmount to prevent memory leaks
  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
      clearPollInterval();
      clearSsoPolling();
    };
  }, []);

  const checkDependencies = async () => {
    try {
      const deps = await invoke<Record<string, DependencyStatus>>("check_dependencies");
      setDependencies(deps);
    } catch (e) {
      console.error("Failed to check dependencies:", e);
    }
  };

  const loadTemplates = async () => {
    try {
      const tmpl = await invoke<Template[]>("get_templates");
      setTemplates(tmpl);
    } catch (e) {
      console.error("Failed to load templates:", e);
    }
  };

  const loadAzureResourceGroups = async (forceRefresh = false) => {
    const subscriptionId = credentials.azure_subscription_id || "";
    
    // Use cached data if available and not forcing refresh
    if (!forceRefresh && subscriptionId && subscriptionId === azureResourceGroupsCacheKey) {
      return;
    }
    
    try {
      const groups = await invoke<{name: string, location: string}[]>("get_azure_resource_groups");
      if (isMountedRef.current) {
        setAzureResourceGroups(groups);
        setAzureResourceGroupsCacheKey(subscriptionId);
      }
    } catch (e) {
      console.error("Failed to load Azure resource groups:", e);
      if (isMountedRef.current) {
        setAzureResourceGroups([]);
        setAzureResourceGroupsCacheKey("");
      }
    }
  };

  const installTerraform = async () => {
    setInstallingTerraform(true);
    setError(null);
    try {
      await invoke("install_terraform");
      await checkDependencies();
    } catch (e: any) {
      setError(`Failed to install Terraform: ${e}`);
    } finally {
      setInstallingTerraform(false);
    }
  };

  const selectCloud = (cloud: string) => {
    // Set loading state first
    setLoadingCloud(cloud);
    setSelectedCloud(cloud);
    
    // Use setTimeout to ensure React has time to render the loading state
    // before we start the async work
    setTimeout(async () => {
      // Check dependencies after selecting cloud
      await Promise.all([
        checkDependencies(),
        new Promise(resolve => setTimeout(resolve, POLLING.MIN_LOADING_TIME))
      ]);
      
      setLoadingCloud(null);
      setScreen("dependencies");
    }, 50); // Small delay to allow React to paint the loading state
  };

  const selectTemplate = async (template: Template) => {
    // Check if we're re-selecting the same template (e.g., coming back after fixing credentials)
    const isSameTemplate = selectedTemplate?.id === template.id;
    const hasExistingValues = Object.keys(formValues).length > 0;
    
    setSelectedTemplate(template);
    setLoading(true);
    setFormSubmitAttempted(false); // Reset form validation state
    try {
      const vars = await invoke<TerraformVariable[]>("get_template_variables", {
        templateId: template.id,
      });
      setVariables(vars);
      
      // Only reset form values if selecting a different template
      if (!isSameTemplate || !hasExistingValues) {
        // Initialize form values with defaults for new template
        const defaults: Record<string, any> = {};
        const randomSuffix = generateRandomSuffix();
        const shortSuffix = randomSuffix.replace(/-/g, "").slice(0, 8); // For storage names (no hyphens, shorter)
        
        vars.forEach((v) => {
          if (v.name === "prefix") {
            // Add random suffix to prefix for uniqueness
            const basePrefix = v.default || "databricks";
            defaults[v.name] = `${basePrefix}-${randomSuffix}`;
          } else if (v.name === "vnet_name") {
            defaults[v.name] = `databricks-vnet-${randomSuffix}`;
          } else if (v.name === "vnet_resource_group_name") {
            // Will be filled from resource_group_name later or user can change
            defaults[v.name] = "";
          } else if (v.name === "workspace_name") {
            defaults[v.name] = `databricks-ws-${randomSuffix}`;
          } else if (v.name === "root_storage_name") {
            // Storage names: lowercase alphanumeric only, 3-24 chars
            defaults[v.name] = `dbstorage${shortSuffix}`;
          } else if (v.name === "subnet_public_cidr") {
            // Default public subnet within 10.0.0.0/20
            defaults[v.name] = "10.0.0.0/22";
          } else if (v.name === "subnet_private_cidr") {
            // Default private subnet within 10.0.0.0/20
            defaults[v.name] = "10.0.4.0/22";
          } else if (v.name === "location") {
            // Default to a common Azure region
            defaults[v.name] = "eastus2";
          } else if (v.name === "admin_user" && azureAccount?.user) {
            // Auto-fill admin user from Azure login
            defaults[v.name] = azureAccount.user;
          } else if (v.name === "create_new_resource_group") {
            // Default to creating new RG, will be set to false if user selects existing
            defaults[v.name] = true;
          } else if (v.default !== null) {
            defaults[v.name] = v.default;
          } else {
            defaults[v.name] = "";
          }
        });
        setFormValues(defaults);
        setTagPairs([]); // Reset tags for new template
      }
      
      // Load Azure resource groups if deploying to Azure
      if (selectedCloud === CLOUDS.AZURE) {
        loadAzureResourceGroups();
      }
    } catch (e: any) {
      setError(`Failed to load template: ${e}`);
    } finally {
      setLoading(false);
    }
    setScreen("configuration");
  };

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

  // Tag management helpers
  const updateTagsFormValue = (pairs: {key: string, value: string}[]) => {
    // Convert tag pairs to JSON format for the backend to parse
    const validPairs = pairs.filter(p => p.key.trim() !== "");
    if (validPairs.length === 0) {
      setFormValues(prev => ({ ...prev, tags: "" }));
    } else {
      // Create a JSON object string that the backend can parse
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

  const handleCredentialChange = (name: string, value: string) => {
    setCredentials((prev) => ({ ...prev, [name]: value }));
  };

  // Clear sensitive credential data (secrets only, keep IDs for display)
  const clearSensitiveCredentials = useCallback(() => {
    setCredentials(prev => ({
      ...prev,
      databricks_client_secret: undefined,
      aws_secret_access_key: undefined,
      aws_session_token: undefined,
      azure_client_secret: undefined,
    }));
  }, []);

  // AWS authentication functions
  const loadAwsProfiles = async () => {
    try {
      const profiles = await invoke<AwsProfile[]>("get_aws_profiles");
      setAwsProfiles(profiles);
      if (profiles.length > 0) {
        // If profiles exist and none selected, select default or first
        if (!credentials.aws_profile) {
          const defaultProfile = profiles.find(p => p.name === "default") || profiles[0];
          setCredentials(prev => ({ ...prev, aws_profile: defaultProfile.name }));
          // Auto-check identity for the default profile
          checkAwsIdentity(defaultProfile.name);
        }
      } else {
        // No profiles - switch to access keys mode
        setAwsAuthMode("keys");
      }
    } catch (e) {
      console.error("Failed to load AWS profiles:", e);
      setAwsAuthMode("keys");
    }
  };

  const checkAwsIdentity = async (profile: string) => {
    setAwsLoading(true);
    setAwsAuthError(null);
    setAwsIdentity(null);
    try {
      const identity = await invoke<AwsIdentity>("get_aws_identity", { profile });
      setAwsIdentity(identity);
    } catch (e: any) {
      setAwsAuthError(e.toString());
    } finally {
      setAwsLoading(false);
    }
  };

  const handleAwsProfileChange = (profile: string) => {
    setCredentials(prev => ({ ...prev, aws_profile: profile }));
    setAwsIdentity(null);
    setAwsAuthError(null);
    checkAwsIdentity(profile);
  };

  const handleAwsSsoLogin = async () => {
    const profile = credentials.aws_profile || "default";
    setAwsLoading(true);
    setAwsAuthError(null);
    clearSsoPolling();
    
    try {
      await invoke("aws_sso_login", { profile });
      
      // Poll for SSO completion
      let attempts = 0;
      ssoPollingRef.current = setInterval(async () => {
        attempts++;
        try {
          const identity = await invoke<AwsIdentity>("get_aws_identity", { profile });
          clearSsoPolling();
          if (isMountedRef.current) {
            setAwsIdentity(identity);
            setAwsLoading(false);
          }
        } catch {
          if (attempts >= POLLING.SSO_MAX_ATTEMPTS) {
            clearSsoPolling();
            if (isMountedRef.current) {
              setAwsAuthError("SSO authentication timed out. Please try again.");
              setAwsLoading(false);
            }
          }
        }
      }, POLLING.SSO_CHECK_INTERVAL);
    } catch (e: any) {
      setAwsAuthError(e.toString());
      setAwsLoading(false);
    }
  };

  // Azure helper functions
  const checkAzureAccount = async () => {
    setAzureLoading(true);
    setAzureAuthError(null);
    setAzureAccount(null);
    try {
      const account = await invoke<AzureAccount>("get_azure_account");
      setAzureAccount(account);
      // Also update credentials with tenant and subscription
      setCredentials(prev => ({
        ...prev,
        azure_tenant_id: account.tenant_id,
        azure_subscription_id: account.subscription_id,
      }));
      // Load subscriptions
      loadAzureSubscriptions();
    } catch (e: any) {
      setAzureAuthError(e.toString());
    } finally {
      setAzureLoading(false);
    }
  };

  const loadAzureSubscriptions = async () => {
    try {
      const subs = await invoke<AzureSubscription[]>("get_azure_subscriptions");
      setAzureSubscriptions(subs);
    } catch (e) {
      console.error("Failed to load Azure subscriptions:", e);
    }
  };

  const handleAzureLogin = async () => {
    setAzureLoading(true);
    setAzureAuthError(null);
    clearSsoPolling();
    
    try {
      await invoke("azure_login");
      
      // Poll for Azure login completion
      let attempts = 0;
      ssoPollingRef.current = setInterval(async () => {
        attempts++;
        try {
          const account = await invoke<AzureAccount>("get_azure_account");
          clearSsoPolling();
          if (isMountedRef.current) {
            setAzureAccount(account);
            setCredentials(prev => ({
              ...prev,
              azure_tenant_id: account.tenant_id,
              azure_subscription_id: account.subscription_id,
            }));
            loadAzureSubscriptions();
            setAzureLoading(false);
          }
        } catch {
          if (attempts >= POLLING.SSO_MAX_ATTEMPTS) {
            clearSsoPolling();
            if (isMountedRef.current) {
              setAzureAuthError("Azure authentication timed out. Please try again.");
              setAzureLoading(false);
            }
          }
        }
      }, POLLING.SSO_CHECK_INTERVAL);
    } catch (e: any) {
      setAzureAuthError(e.toString());
      setAzureLoading(false);
    }
  };

  const handleAzureSubscriptionChange = async (subscriptionId: string) => {
    const sub = azureSubscriptions.find(s => s.id === subscriptionId);
    if (sub) {
      setCredentials(prev => ({
        ...prev,
        azure_subscription_id: sub.id,
        azure_tenant_id: sub.tenant_id,
      }));
      try {
        await invoke("set_azure_subscription", { subscriptionId });
        // Force refresh resource groups for new subscription
        loadAzureResourceGroups(true);
      } catch (e) {
        console.error("Failed to set subscription:", e);
      }
    }
  };

  const cancelDeployment = async () => {
    try {
      await invoke("cancel_deployment");
      const status = await invoke<DeploymentStatus>("get_deployment_status");
      setDeploymentStatus(status);
    } catch (e: any) {
      setError(`Failed to cancel: ${e}`);
    }
  };

  const rollback = async () => {
    if (!deploymentName) return;
    clearPollInterval();

    try {
      // Set up UI to show cleanup progress
      setIsRollingBack(true);
      setDeploymentStep("deploying");
      setShowDetailedLogs(true);
      setError(null);
      
      // Start the rollback (destroy)
      await invoke("rollback_deployment", {
        deploymentName: deploymentName,
        credentials,
      });

      // Poll for status updates while running
      pollIntervalRef.current = setInterval(async () => {
        try {
          const status = await invoke<DeploymentStatus>("get_deployment_status");
          // Only update state if component is still mounted
          if (!isMountedRef.current) return;
          
          setDeploymentStatus(status);
          
          if (!status.running) {
            clearPollInterval();
            if (status.success) {
              // Keep isRollingBack=true so completion screen shows "Cleanup Complete"
              setDeploymentStep("complete");
            } else {
              setIsRollingBack(false);
              setDeploymentStep("failed");
            }
          }
        } catch {
          clearPollInterval();
          if (isMountedRef.current) {
            setIsRollingBack(false);
          }
        }
      }, POLLING.ROLLBACK_INTERVAL);
    } catch (e: any) {
      setError(`Failed to cleanup: ${e}`);
      setDeploymentStep("failed");
      setIsRollingBack(false);
    }
  };

  const goBack = () => {
    setError(null);
    setAwsAuthError(null);
    setAzureAuthError(null);
    switch (screen) {
      case "cloud-selection":
        setScreen("welcome");
        clearSensitiveCredentials();
        break;
      case "dependencies":
        setScreen("cloud-selection");
        setSelectedCloud("");
        break;
      case "databricks-credentials":
        setScreen("dependencies");
        break;
      case "aws-credentials":
        setScreen("databricks-credentials");
        break;
      case "azure-credentials":
        setScreen("databricks-credentials");
        break;
      case "template-selection":
        if (selectedCloud === CLOUDS.AWS) {
          setScreen("aws-credentials");
        } else if (selectedCloud === CLOUDS.AZURE) {
          setScreen("azure-credentials");
        } else {
          setScreen("databricks-credentials");
        }
        break;
      case "configuration":
        setScreen("template-selection");
        break;
      case "deployment":
        if (!deploymentStatus?.running) {
          setScreen("configuration");
          setDeploymentStep("ready");
        }
        break;
    }
  };

  // Continue from dependencies to Databricks credentials screen
  const continueFromDependencies = async () => {
    try {
      const creds = await invoke<CloudCredentials>("get_cloud_credentials", { cloud: selectedCloud });
      setCredentials(creds);
    } catch (e) {
      console.error("Failed to get credentials:", e);
    }
    setScreen("databricks-credentials");
  };

  // Validate and continue from Databricks credentials
  const validateAndContinueFromCredentials = async () => {
    // Validate required Databricks fields
    if (!credentials.databricks_account_id?.trim()) {
      setError("Databricks Account ID is required");
      return;
    }
    if (!credentials.databricks_client_id?.trim()) {
      setError("Databricks Client ID is required");
      return;
    }
    if (!credentials.databricks_client_secret?.trim()) {
      setError("Databricks Client Secret is required");
      return;
    }

    setError(null);
    setValidatingCredentials(true);

    try {
      // Call backend to validate Databricks credentials
      await invoke("validate_databricks_credentials", {
        accountId: credentials.databricks_account_id,
        clientId: credentials.databricks_client_id,
        clientSecret: credentials.databricks_client_secret,
        cloud: selectedCloud,
      });
      
      setValidatingCredentials(false);
      
      // Navigate to cloud-specific credentials screen
      if (selectedCloud === CLOUDS.AWS) {
        loadAwsProfiles();
        setScreen("aws-credentials");
      } else if (selectedCloud === CLOUDS.AZURE) {
        checkAzureAccount();
        setScreen("azure-credentials");
      } else {
        setScreen("template-selection");
      }
    } catch (e: any) {
      setValidatingCredentials(false);
      setError(`Invalid Databricks credentials: ${e}`);
    }
  };

  // Validate and continue from AWS credentials
  const validateAndContinueFromAwsCredentials = async () => {
    setAwsAuthError(null);
    
    if (awsAuthMode === "profile") {
      // For profile mode, check if we have a valid identity
      if (!awsIdentity) {
        setAwsAuthError("Please verify your AWS credentials first");
        return;
      }
    } else {
      // For access keys mode, validate required fields
      if (!credentials.aws_access_key_id?.trim()) {
        setAwsAuthError("AWS Access Key ID is required");
        return;
      }
      if (!credentials.aws_secret_access_key?.trim()) {
        setAwsAuthError("AWS Secret Access Key is required");
        return;
      }
    }
    
    setScreen("template-selection");
  };

  // Validate and continue from Azure credentials
  const validateAndContinueFromAzureCredentials = async () => {
    setAzureAuthError(null);
    
    if (azureAuthMode === "cli") {
      // For CLI mode, check if we have a valid account
      if (!azureAccount) {
        setAzureAuthError("Please verify your Azure credentials first");
        return;
      }
      if (!credentials.azure_subscription_id) {
        setAzureAuthError("Please select an Azure subscription");
        return;
      }
    } else {
      // For service principal mode, validate required fields
      if (!credentials.azure_tenant_id?.trim()) {
        setAzureAuthError("Azure Tenant ID is required");
        return;
      }
      if (!credentials.azure_subscription_id?.trim()) {
        setAzureAuthError("Azure Subscription ID is required");
        return;
      }
      if (!credentials.azure_client_id?.trim()) {
        setAzureAuthError("Azure Client ID is required");
        return;
      }
      if (!credentials.azure_client_secret?.trim()) {
        setAzureAuthError("Azure Client Secret is required");
        return;
      }
    }
    
    setScreen("template-selection");
  };

  // Render screens
  const renderWelcome = () => (
    <div className="container">
      <div className="welcome-content">
        <h1 className="gradient" style={{ fontSize: "3em", marginBottom: "20px" }}>
          Databricks Deployer
        </h1>
        <p style={{ fontSize: "1.3em", color: "#b3b3b3", fontStyle: "italic", textAlign: "left" }}>
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
            onClick={openDeploymentsFolder}
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

  const renderDependencies = () => {
    const terraformDep = dependencies["terraform"];
    const canContinue = terraformDep?.installed;
    
    // Get the relevant cloud CLI based on selection
    const cloudCliKey = selectedCloud === CLOUDS.AWS ? "aws" : selectedCloud === CLOUDS.AZURE ? "azure" : "gcloud";
    const cloudCliDep = dependencies[cloudCliKey];
    const cloudCliName = selectedCloud === CLOUDS.AWS ? "AWS CLI" : selectedCloud === CLOUDS.AZURE ? "Azure CLI" : "Google Cloud CLI";

    return (
      <div className="container">
        <button className="back-btn" onClick={goBack}>
          ← Back
        </button>
        <h1>System Requirements for {CLOUD_DISPLAY_NAMES[selectedCloud] || selectedCloud}</h1>
        <p className="subtitle">
          Let's make sure your system has everything needed for your {CLOUD_DISPLAY_NAMES[selectedCloud] || selectedCloud} deployment.
        </p>

        {error && <div className="alert alert-error">{error}</div>}

        <div className="dependency-list">
          {/* Terraform - Required */}
          <div className="dependency-item">
            <div className="dependency-info">
              <div className={`dependency-status ${terraformDep?.installed ? "installed" : "missing"}`} />
              <div>
                <div className="dependency-name">Terraform</div>
                {terraformDep?.version && (
                  <div className="dependency-version">v{terraformDep.version}</div>
                )}
              </div>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
              <span className="dependency-badge required">Required</span>
              {!terraformDep?.installed && (
                <button
                  className="btn"
                  onClick={installTerraform}
                  disabled={installingTerraform}
                >
                  {installingTerraform ? (
                    <>
                      <span className="spinner" />
                      Installing...
                    </>
                  ) : (
                    "Install"
                  )}
                </button>
              )}
            </div>
          </div>

          {/* Cloud-specific CLI - Optional */}
          <div className="dependency-item">
            <div className="dependency-info">
              <div className={`dependency-status ${cloudCliDep?.installed ? "installed" : "optional"}`} />
              <div>
                <div className="dependency-name">{cloudCliName}</div>
                {cloudCliDep?.version && (
                  <div className="dependency-version">{cloudCliDep.version}</div>
                )}
              </div>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
              <span className="dependency-badge">Optional</span>
              {!cloudCliDep?.installed && (
                <a href={cloudCliDep?.install_url} target="_blank" className="btn btn-secondary btn-small">
                  Install Guide
                </a>
              )}
            </div>
          </div>

        </div>

        {!terraformDep?.installed && (
          <div className="alert alert-warning">
            Terraform is required to deploy workspaces. Click "Install" above to automatically
            download and install it, or install it manually from{" "}
            <a href={terraformDep?.install_url} target="_blank" style={{ color: "#ffb347" }}>
              terraform.io
            </a>
          </div>
        )}

        <div className="alert alert-info">
          {cloudCliName} is optional. If installed, credentials will be auto-detected.
          Otherwise, you can enter them manually in the next steps.
        </div>

        <div style={{ marginTop: "32px" }}>
          <button className="btn" onClick={continueFromDependencies} disabled={!canContinue}>
            Continue →
          </button>
        </div>
      </div>
    );
  };

  const renderDatabricksCredentials = () => {
    const canContinue = 
      credentials.databricks_account_id?.trim() && 
      credentials.databricks_client_id?.trim() && 
      credentials.databricks_client_secret?.trim();

    return (
      <div className="container">
        <button className="back-btn" onClick={goBack}>
          ← Back
        </button>
        <h1>Databricks Account Credentials</h1>
        <p className="subtitle">
          Enter your Databricks account credentials. A service principal with account admin privileges is required.
        </p>

        {error && <div className="alert alert-error">{error}</div>}

        <div className="form-section">
          <h3>Databricks Account Console</h3>
          <p style={{ color: "#888", marginBottom: "20px" }}>
            You can find these credentials in your{" "}
            <a 
              href={selectedCloud === CLOUDS.AZURE ? "https://accounts.azuredatabricks.net" : "https://accounts.cloud.databricks.com"} 
              target="_blank" 
              style={{ color: "#ff6b35" }}
            >
              Databricks Account Console
            </a>
            . You'll need a service principal with account admin privileges.
          </p>
          
          <div className="form-group">
            <label>Account ID *</label>
            <input
              type="text"
              autoCapitalize="none"
              autoCorrect="off"
              spellCheck={false}
              value={credentials.databricks_account_id || ""}
              onChange={(e) => handleCredentialChange("databricks_account_id", e.target.value)}
              placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
            />
            <div className="help-text">
              Found in Databricks Account Console. Open your top‑right user menu; you'll see a copy button for your Account ID there.
            </div>
          </div>

          <div className="two-column">
            <div className="form-group">
              <label>Client ID (Service Principal) *</label>
              <input
                type="text"
                autoCapitalize="none"
                autoCorrect="off"
                spellCheck={false}
                value={credentials.databricks_client_id || ""}
                onChange={(e) => handleCredentialChange("databricks_client_id", e.target.value)}
                placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
              />
              <div className="help-text">
                Service Principal's Application ID
              </div>
            </div>
            <div className="form-group">
              <label>Client Secret *</label>
              <input
                type="password"
                value={credentials.databricks_client_secret || ""}
                onChange={(e) => handleCredentialChange("databricks_client_secret", e.target.value)}
                placeholder="Enter service principal secret"
              />
              <div className="help-text">
                Service Principal's OAuth secret
              </div>
            </div>
          </div>
        </div>

        <div className="alert alert-info">
          <strong>Don't have a service principal?</strong> In the Databricks Account Console, go to 
          User Management → Service Principals → Add service principal. Then generate an OAuth secret 
          and grant it account admin role.
        </div>

        <div style={{ marginTop: "32px" }}>
          <button 
            className="btn" 
            onClick={validateAndContinueFromCredentials} 
            disabled={!canContinue || validatingCredentials}
          >
            {validatingCredentials ? (
              <>
                <span className="spinner" />
                Validating...
              </>
            ) : (
              "Validate & Continue →"
            )}
          </button>
        </div>
      </div>
    );
  };

  const renderAwsCredentials = () => {
    const canContinue = awsAuthMode === "profile" 
      ? !!awsIdentity 
      : !!(credentials.aws_access_key_id?.trim() && credentials.aws_secret_access_key?.trim());

    return (
      <div className="container">
        <button className="back-btn" onClick={goBack}>
          ← Back
        </button>
        <h1>AWS Credentials</h1>
        <p className="subtitle">
          Configure your AWS credentials for deploying resources.
        </p>

        {awsAuthError && <div className="alert alert-error">{awsAuthError}</div>}

        <div className="form-section">
          <h3>Authentication Method</h3>
          
          <div className="auth-mode-selector">
            <label className="radio-label">
              <input
                type="radio"
                checked={awsAuthMode === "profile"}
                onChange={() => setAwsAuthMode("profile")}
              />
              Use AWS CLI Profile (recommended)
            </label>
            <label className="radio-label">
              <input
                type="radio"
                checked={awsAuthMode === "keys"}
                onChange={() => setAwsAuthMode("keys")}
              />
              Use Access Keys
            </label>
          </div>

          {awsAuthMode === "profile" && (
            <>
              <div className="alert alert-info" style={{ marginBottom: "16px" }}>
                <strong>How to set up AWS CLI profiles:</strong>
                <ol style={{ margin: "8px 0 0 0", paddingLeft: "20px", fontSize: "13px" }}>
                  <li>Install the <a href="https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html" target="_blank" style={{ color: "#ff6b35" }}>AWS CLI</a></li>
                  <li>Run <code>aws configure</code> (for access keys) or <code>aws configure sso</code> (for SSO)</li>
                  <li>Enter your credentials when prompted</li>
                </ol>
                <p style={{ margin: "8px 0 0 0", fontSize: "12px", color: "#888" }}>
                  Profiles are stored in <code>~/.aws/config</code> and <code>~/.aws/credentials</code>.{" "}
                  <a href="https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-files.html" target="_blank" style={{ color: "#ff6b35" }}>
                    Learn more →
                  </a>
                </p>
              </div>
              {awsProfiles.length === 0 ? (
                <div className="alert alert-warning">
                  <strong>No AWS profiles found.</strong>
                  <p style={{ margin: "8px 0 0 0", fontSize: "13px" }}>
                    Please set up AWS CLI following the instructions above, or switch to "Use Access Keys".
                  </p>
                </div>
              ) : (
                <div className="two-column">
                  <div className="form-group">
                    <label>AWS Profile</label>
                    <select
                      value={credentials.aws_profile || ""}
                      onChange={(e) => handleAwsProfileChange(e.target.value)}
                    >
                      {awsProfiles.map((p) => (
                        <option key={p.name} value={p.name}>
                          {p.name}{p.is_sso ? " (SSO)" : ""}
                        </option>
                      ))}
                    </select>
                  </div>
                  <div className="form-group">
                    <label>Status</label>
                    <div className="aws-status">
                      {awsLoading && <span className="spinner" />}
                      {awsIdentity && (
                        <span className="success">
                          Account: {awsIdentity.account}
                        </span>
                      )}
                      {awsAuthError && !awsLoading && (
                        <span className="error">{awsAuthError}</span>
                      )}
                      {!awsIdentity && !awsAuthError && !awsLoading && (
                        <span style={{ color: "#888" }}>Click Check to verify</span>
                      )}
                    </div>
                    <div style={{ marginTop: "8px", display: "flex", gap: "8px" }}>
                      <button
                        type="button"
                        className="btn btn-small btn-secondary"
                        onClick={() => checkAwsIdentity(credentials.aws_profile || "default")}
                        disabled={awsLoading}
                      >
                        {awsLoading ? "Checking..." : "Check"}
                      </button>
                      {awsProfiles.find(p => p.name === credentials.aws_profile)?.is_sso && (
                        <button
                          type="button"
                          className="btn btn-small"
                          onClick={handleAwsSsoLogin}
                          disabled={awsLoading}
                        >
                          SSO Login
                        </button>
                      )}
                    </div>
                  </div>
                </div>
              )}
              {awsIdentity && (
                <div className="alert alert-success" style={{ marginTop: "16px" }}>
                  Authenticated as: <strong>{awsIdentity.arn}</strong>
                </div>
              )}
            </>
          )}

          {awsAuthMode === "keys" && (
            <>
              <div className="alert alert-info" style={{ marginBottom: "16px" }}>
                <strong>How to get AWS Access Keys:</strong>
                <ol style={{ margin: "8px 0 0 0", paddingLeft: "20px", fontSize: "13px" }}>
                  <li>Go to <a href="https://console.aws.amazon.com/iam/home#/security_credentials" target="_blank" style={{ color: "#ff6b35" }}>AWS IAM Console → Security Credentials</a></li>
                  <li>Click "Create access key" under Access Keys section</li>
                  <li>Copy the Access Key ID and Secret Access Key</li>
                </ol>
                <p style={{ margin: "8px 0 0 0", fontSize: "12px", color: "#ffb347" }}>
                  ⚠️ Access keys are long-lived credentials. Consider using AWS CLI profiles with SSO for better security.
                </p>
              </div>
              <div className="two-column">
                <div className="form-group">
                  <label>AWS Access Key ID *</label>
                  <input
                    type="text"
                    autoCapitalize="none"
                    autoCorrect="off"
                    spellCheck={false}
                    value={credentials.aws_access_key_id || ""}
                    onChange={(e) => handleCredentialChange("aws_access_key_id", e.target.value)}
                    placeholder="AKIA..."
                  />
                </div>
                <div className="form-group">
                  <label>AWS Secret Access Key *</label>
                  <input
                    type="password"
                    value={credentials.aws_secret_access_key || ""}
                    onChange={(e) => handleCredentialChange("aws_secret_access_key", e.target.value)}
                    placeholder="Enter secret key"
                  />
                </div>
              </div>
              <div className="form-group">
                <label>AWS Session Token (optional)</label>
                <input
                  type="password"
                  value={credentials.aws_session_token || ""}
                  onChange={(e) => handleCredentialChange("aws_session_token", e.target.value)}
                  placeholder="For temporary credentials"
                />
                <div className="help-text">Only needed for temporary credentials (e.g., from STS AssumeRole)</div>
              </div>
            </>
          )}
        </div>

        <div style={{ marginTop: "32px" }}>
          <button 
            className="btn" 
            onClick={validateAndContinueFromAwsCredentials} 
            disabled={!canContinue}
          >
            Continue →
          </button>
        </div>
      </div>
    );
  };

  const renderAzureCredentials = () => {
    const canContinue = azureAuthMode === "cli" 
      ? !!(azureAccount && credentials.azure_subscription_id) 
      : !!(credentials.azure_tenant_id?.trim() && credentials.azure_subscription_id?.trim() && 
           credentials.azure_client_id?.trim() && credentials.azure_client_secret?.trim());

    return (
      <div className="container">
        <button className="back-btn" onClick={goBack}>
          ← Back
        </button>
        <h1>Azure Credentials</h1>
        <p className="subtitle">
          Configure your Azure credentials for deploying resources.
        </p>

        {azureAuthError && <div className="alert alert-error">{azureAuthError}</div>}

        <div className="form-section">
          <h3>Authentication Method</h3>
          
          <div className="auth-mode-selector">
            <label className="radio-label">
              <input
                type="radio"
                checked={azureAuthMode === "cli"}
                onChange={() => setAzureAuthMode("cli")}
              />
              Use Azure CLI (recommended)
            </label>
            <label className="radio-label">
              <input
                type="radio"
                checked={azureAuthMode === "service_principal"}
                onChange={() => setAzureAuthMode("service_principal")}
              />
              Use Service Principal
            </label>
          </div>

          {azureAuthMode === "cli" && (
            <>
              <div className="help-text" style={{ marginBottom: "16px" }}>
                Azure CLI credentials are managed via <code>az login</code>.{" "}
                <a href="https://docs.microsoft.com/en-us/cli/azure/authenticate-azure-cli" target="_blank" style={{ color: "#ff6b35" }}>
                  Learn more
                </a>
              </div>
              
              <div className="form-group">
                <label>Status</label>
                <div className="aws-status">
                  {azureLoading && <span className="spinner" />}
                  {azureAccount && (
                    <span className="success">
                      Logged in as: {azureAccount.user}
                    </span>
                  )}
                  {azureAuthError && !azureLoading && !azureAccount && (
                    <span className="error">{azureAuthError}</span>
                  )}
                  {!azureAccount && !azureAuthError && !azureLoading && (
                    <span style={{ color: "#888" }}>Click Check or Login to verify</span>
                  )}
                </div>
                <div style={{ marginTop: "8px", display: "flex", gap: "8px" }}>
                  <button
                    type="button"
                    className="btn btn-small btn-secondary"
                    onClick={checkAzureAccount}
                    disabled={azureLoading}
                  >
                    {azureLoading ? "Checking..." : "Check"}
                  </button>
                  <button
                    type="button"
                    className="btn btn-small"
                    onClick={handleAzureLogin}
                    disabled={azureLoading}
                  >
                    Login
                  </button>
                </div>
              </div>

              {azureAccount && azureSubscriptions.length > 0 && (
                <div className="form-group" style={{ marginTop: "16px" }}>
                  <label>Subscription</label>
                  <select
                    value={credentials.azure_subscription_id || ""}
                    onChange={(e) => handleAzureSubscriptionChange(e.target.value)}
                  >
                    <option value="">Select a subscription...</option>
                    {azureSubscriptions.map((sub) => (
                      <option key={sub.id} value={sub.id}>
                        {sub.name} {sub.is_default ? "(default)" : ""}
                      </option>
                    ))}
                  </select>
                  <div className="help-text">Select the Azure subscription to deploy resources to</div>
                </div>
              )}

              {azureAccount && (
                <div className="alert alert-success" style={{ marginTop: "16px" }}>
                  Using subscription: <strong>{azureAccount.subscription_name}</strong>
                  <br />
                  <span style={{ fontSize: "12px", opacity: 0.8 }}>Tenant: {azureAccount.tenant_id}</span>
                </div>
              )}
            </>
          )}

          {azureAuthMode === "service_principal" && (
            <>
              <div className="alert alert-warning" style={{ marginBottom: "16px" }}>
                Service principal credentials require manual management. Use Azure CLI when possible.
              </div>
              <div className="two-column">
                <div className="form-group">
                  <label>Tenant ID *</label>
                  <input
                    type="text"
                    autoCapitalize="none"
                    autoCorrect="off"
                    spellCheck={false}
                    value={credentials.azure_tenant_id || ""}
                    onChange={(e) => handleCredentialChange("azure_tenant_id", e.target.value)}
                    placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
                  />
                  <div className="help-text">Found in Azure Portal → Microsoft Entra ID</div>
                </div>
                <div className="form-group">
                  <label>Subscription ID *</label>
                  <input
                    type="text"
                    autoCapitalize="none"
                    autoCorrect="off"
                    spellCheck={false}
                    value={credentials.azure_subscription_id || ""}
                    onChange={(e) => handleCredentialChange("azure_subscription_id", e.target.value)}
                    placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
                  />
                  <div className="help-text">Found in Azure Portal → Subscriptions</div>
                </div>
              </div>
              <div className="two-column">
                <div className="form-group">
                  <label>Client ID *</label>
                  <input
                    type="text"
                    autoCapitalize="none"
                    autoCorrect="off"
                    spellCheck={false}
                    value={credentials.azure_client_id || ""}
                    onChange={(e) => handleCredentialChange("azure_client_id", e.target.value)}
                    placeholder="Service Principal Application ID"
                  />
                </div>
                <div className="form-group">
                  <label>Client Secret *</label>
                  <input
                    type="password"
                    value={credentials.azure_client_secret || ""}
                    onChange={(e) => handleCredentialChange("azure_client_secret", e.target.value)}
                    placeholder="Service Principal Secret"
                  />
                </div>
              </div>
            </>
          )}
        </div>

        <div style={{ marginTop: "32px" }}>
          <button 
            className="btn" 
            onClick={validateAndContinueFromAzureCredentials} 
            disabled={!canContinue}
          >
            Continue →
          </button>
        </div>
      </div>
    );
  };

  const renderCloudSelection = () => (
    <div className="container">
      <button className="back-btn" onClick={goBack} disabled={!!loadingCloud}>
        ← Back
      </button>
      <h1>Select Cloud Provider</h1>
      <p className="subtitle">
        Select the cloud platform on which you would like to deploy your Databricks workspace.
      </p>

      {loadingCloud && (
        <div className="loading-overlay">
          <div className="loading-content">
            <span className="spinner large" />
            <div>Checking {CLOUD_DISPLAY_NAMES[loadingCloud || ""] || loadingCloud} dependencies...</div>
          </div>
        </div>
      )}

      <div className="cloud-selection">
        <div 
          className={`cloud-card azure ${loadingCloud ? "disabled" : ""}`}
          onClick={() => !loadingCloud && selectCloud(CLOUDS.AZURE)}
        >
          <div className="cloud-name">Azure</div>
          <div className="cloud-description">
            Deploy on Microsoft Azure with VNet injection and Unity Catalog
          </div>
        </div>

        <div 
          className={`cloud-card aws ${loadingCloud ? "disabled" : ""}`}
          onClick={() => !loadingCloud && selectCloud(CLOUDS.AWS)}
        >
          <div className="cloud-name">AWS</div>
          <div className="cloud-description">
            Deploy on Amazon Web Services with customer-managed VPC and Unity Catalog
          </div>
        </div>

        <div className="cloud-card gcp disabled">
          <div className="cloud-name">GCP</div>
          <div className="cloud-description">
            Deploy on Google Cloud Platform with Unity Catalog
          </div>
          <div className="coming-soon">Coming Soon</div>
        </div>
      </div>
    </div>
  );

  const renderTemplateSelection = () => {
    const cloudTemplates = templates.filter((t) => t.cloud === selectedCloud);

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
          {cloudTemplates.map((template) => (
            <div
              key={template.id}
              className="template-card"
              onClick={() => selectTemplate(template)}
            >
              <div className="template-title">{template.name}</div>
              <div className="template-description">{template.description}</div>
              <div className="template-features">
                <ul>
                  {template.features.map((feature, i) => (
                    <li key={i}>{feature}</li>
                  ))}
                </ul>
              </div>
            </div>
          ))}

          <div className="template-card" style={{ opacity: 0.6, cursor: "not-allowed" }}>
            <div className="coming-soon">Coming Soon</div>
            <div className="template-title">
              Maximum Security {selectedCloud === CLOUDS.AWS ? "PrivateLink" : ""} Workspace
            </div>
            <div className="template-description">
              Enterprise-grade security with {selectedCloud === CLOUDS.AWS ? "AWS PrivateLink" : "Private Link"} and zero internet exposure
            </div>
          </div>
        </div>
      </div>
    );
  };

  const renderConfiguration = () => {
    if (!selectedTemplate || loading) {
      return (
        <div className="container">
          <div className="loading">Loading configuration...</div>
        </div>
      );
    }

    // Group variables by section
    const sections = groupVariablesBySection(variables, selectedCloud);

    return (
      <div className="container">
        <button className="back-btn" onClick={goBack}>
          ← Back to Templates
        </button>
        <h1>Configure Deployment</h1>
        <p className="subtitle">
          Fill in the configuration values for your Databricks workspace deployment.
        </p>

        {error && <div className="alert alert-error">{error}</div>}

        <form onSubmit={(e) => e.preventDefault()}>
          {/* Variable Sections */}
          {Object.entries(sections)
            .filter(([sectionName]) => !sectionName.startsWith("Advanced"))
            .map(([sectionName, sectionVars]) => (
            <div key={sectionName} className={`form-section ${selectedCloud}`}>
              <h3>{sectionName}</h3>
              <div className="two-column">
                {sectionVars
                  // Conditionally filter Azure VNet-related fields based on create_new_vnet
                  .filter((variable) => {
                    // Default to creating new VNet if not set (undefined defaults to true)
                    const createNewVnetValue = formValues.create_new_vnet;
                    const createNewVnet = createNewVnetValue === true || createNewVnetValue === "true" || createNewVnetValue === undefined;
                    // When creating new VNet: hide vnet_name and vnet_resource_group_name (auto-filled from main resource group)
                    if (createNewVnet && variable.name === "vnet_name") return false;
                    if (createNewVnet && variable.name === "vnet_resource_group_name") return false;
                    // When using existing VNet: hide cidr (existing VNet has its own)
                    if (!createNewVnet && variable.name === "cidr") return false;
                    return true;
                  })
                  .map((variable) => (
                  <div key={variable.name} className="form-group" style={variable.name === "tags" ? { gridColumn: "1 / -1" } : undefined}>
                    <label>
                      {formatVariableName(variable.name)}
                      {variable.required && !variable.default && " *"}
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
                              create_new_resource_group: val === "" ? true : false, // Dropdown = existing RG
                            }));
                          }}
                          className={formSubmitAttempted && formValidation.missingFields.includes(variable.name) ? "input-error" : ""}
                        >
                          <option value="">-- Select existing or enter below --</option>
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
                            // Check if typed value matches an existing RG (case-insensitive)
                            const isExisting = azureResourceGroups.some(
                              rg => rg.name.toLowerCase() === val.toLowerCase()
                            );
                            // Use the exact name from Azure if it matches
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

          {/* Advanced Section (collapsible) */}
          {Object.entries(sections)
            .filter(([sectionName]) => sectionName.startsWith("Advanced"))
            .map(([sectionName, sectionVars]) => (
            <div key={sectionName} className={`form-section advanced ${showAdvanced ? "expanded" : ""}`}>
              <h3 
                onClick={() => setShowAdvanced(!showAdvanced)} 
                style={{ cursor: "pointer", display: "flex", alignItems: "center", gap: "8px" }}
              >
                <span style={{ fontSize: "12px" }}>{showAdvanced ? "▼" : "►"}</span>
                Advanced Options
              </h3>
              {showAdvanced && (
                <>
                  <p style={{ color: "#e67e22", marginBottom: "8px", fontSize: "0.85em", fontWeight: "500" }}>
                    For advanced users only. Modify only if you have specific requirements.
                  </p>
                  <p style={{ color: "#888", marginBottom: "16px", fontSize: "0.85em" }}>
                    Leave all fields empty for auto-detection. Unity Catalog metastore is detected 
                    automatically — if one exists in your region, it will be used; otherwise a new one is created.
                  </p>
                  <div className="two-column">
                    {sectionVars.map((variable) => (
                      <div key={variable.name} className="form-group">
                        <label>{formatVariableName(variable.name)}</label>
                        <input
                          type="text"
                          autoCapitalize="none"
                          autoCorrect="off"
                          spellCheck={false}
                          value={formValues[variable.name] || ""}
                          onChange={(e) => handleFormChange(variable.name, e.target.value)}
                          placeholder="Leave empty for auto-detection"
                        />
                      </div>
                    ))}
                  </div>
                </>
              )}
            </div>
          ))}

          <div style={{ marginTop: "32px" }}>
            <button 
              className="btn btn-large btn-success" 
              onClick={() => {
                if (!formValidation.isValid) {
                  setFormSubmitAttempted(true);
                } else {
                  startDeploymentWizard();
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
                "Create Workspace →"
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
  };

  // Start the deployment wizard - runs init, then plan, then shows review
  const startDeploymentWizard = async () => {
    if (!selectedTemplate) return;

    setLoading(true);
    setError(null);
    setDeploymentStep("initializing");
    setIsRollingBack(false);

    try {
      // Use template name + prefix for the deployment folder name
      const prefix = formValues.prefix || `deployment-${Date.now()}`;
      const depName = `${selectedTemplate.id}-${prefix}`;
      setDeploymentName(depName);
      
      // Save configuration - this creates the deployment folder and returns its path
      const path = await invoke<string>("save_configuration", {
        templateId: selectedTemplate.id,
        deploymentName: depName,
        values: formValues,
        credentials: credentials,
      });
      setTemplatePath(path);

      // Go to deployment screen and run init
      setScreen("deployment");
      
      await invoke("run_terraform_command", {
        deploymentName: depName,
        command: "init",
        credentials,
      });

      // Poll for init completion, then run plan
      pollAndContinueWizard("initializing");
    } catch (e: any) {
      setError(`Failed to start: ${e}`);
      setDeploymentStep("failed");
      setLoading(false);
    }
  };

  // Poll deployment status and continue wizard steps
  const pollAndContinueWizard = useCallback((currentStep: DeploymentStep) => {
    clearPollInterval();
    
    pollIntervalRef.current = setInterval(async () => {
      try {
        const status = await invoke<DeploymentStatus>("get_deployment_status");
        
        // Only update state if component is still mounted
        if (!isMountedRef.current) {
          clearPollInterval();
          return;
        }
        
        setDeploymentStatus(status);

        if (!status.running) {
          clearPollInterval();
          
          if (status.success) {
            // Move to next step based on what we were doing
            if (currentStep === "initializing") {
              setDeploymentStep("planning");
              // Run plan
              const depName = deploymentNameRef.current;
              const creds = credentialsRef.current;
              if (depName) {
                try {
                  await invoke("run_terraform_command", {
                    deploymentName: depName,
                    command: "plan",
                    credentials: creds,
                  });
                  pollAndContinueWizard("planning");
                } catch (e) {
                  console.error("Failed to run plan:", e);
                  setDeploymentStep("failed");
                  setLoading(false);
                }
              }
            } else if (currentStep === "planning") {
              setDeploymentStep("review");
              setLoading(false);
            } else if (currentStep === "deploying") {
              setDeploymentStep("complete");
              setLoading(false);
            }
          } else {
            setDeploymentStep("failed");
            setLoading(false);
          }
        }
      } catch (e) {
        console.error("Failed to poll status:", e);
      }
    }, POLLING.STATUS_INTERVAL);
  }, []);

  // Confirm and deploy
  const confirmAndDeploy = async () => {
    if (!deploymentName) return;
    
    setLoading(true);
    setDeploymentStep("deploying");

    try {
      await invoke("run_terraform_command", {
        deploymentName: deploymentName,
        command: "apply",
        credentials,
      });
      pollAndContinueWizard("deploying");
    } catch (e: any) {
      setError(`Failed to deploy: ${e}`);
      setDeploymentStep("failed");
      setLoading(false);
    }
  };

  // Memoized validation for required form fields
  const formValidation = useMemo(() => {
    // Get required variables (those with required=true and no default value)
    // Exclude credential variables - they are injected from the credentials screens
    const requiredVars = variables.filter(v => 
      v.required && 
      !v.default && 
      !EXCLUDE_VARIABLES.includes(v.name as any)
    );
    
    // Find which required fields are missing (empty or undefined)
    const missingFields = requiredVars.filter(v => {
      const value = formValues[v.name];
      // Consider empty string, undefined, or null as missing
      return value === undefined || value === null || value === "";
    });
    
    // Validate prefix/workspace name format
    const fieldErrors: Record<string, string> = {};
    const prefixValue = formValues["prefix"];
    if (prefixValue) {
      // Workspace names must be lowercase alphanumeric with hyphens, 3-30 chars
      if (!/^[a-z0-9][a-z0-9-]*[a-z0-9]$|^[a-z0-9]$/.test(prefixValue)) {
        fieldErrors["prefix"] = "Must start and end with a letter or number, contain only lowercase letters, numbers, and hyphens";
      } else if (prefixValue.length < 3 || prefixValue.length > 30) {
        fieldErrors["prefix"] = "Must be between 3 and 30 characters";
      } else if (/--/.test(prefixValue)) {
        fieldErrors["prefix"] = "Cannot contain consecutive hyphens";
      }
    }
    
    return {
      isValid: missingFields.length === 0 && Object.keys(fieldErrors).length === 0,
      missingFields: missingFields.map(v => v.name),
      requiredFields: requiredVars.map(v => v.name),
      fieldErrors,
    };
  }, [variables, formValues]);

  // Memoized helper to count resources from terraform output
  const resourceCounts = useMemo(() => {
    if (!deploymentStatus?.output) return null;
    
    const output = deploymentStatus.output;
    // Match unique resource lines - format: "resource_name: Creating..."
    // Use a Set to count unique resources (in case of retries)
    const creatingSet = new Set<string>();
    const createdSet = new Set<string>();
    const destroyingSet = new Set<string>();
    const destroyedSet = new Set<string>();
    
    const lines = output.split('\n');
    for (const line of lines) {
      // Match "resource_name: Creating..."
      const creatingMatch = line.match(/^([^:]+):\s*Creating\.\.\.$/);
      if (creatingMatch) {
        creatingSet.add(creatingMatch[1].trim());
      }
      
      // Match "resource_name: Creation complete"
      const createdMatch = line.match(/^([^:]+):\s*Creation complete/);
      if (createdMatch) {
        createdSet.add(createdMatch[1].trim());
      }
      
      // Match "resource_name: Destroying..."
      const destroyingMatch = line.match(/^([^:]+):\s*Destroying\.\.\.$/);
      if (destroyingMatch) {
        destroyingSet.add(destroyingMatch[1].trim());
      }
      
      // Match "resource_name: Destruction complete"
      const destroyedMatch = line.match(/^([^:]+):\s*Destruction complete/);
      if (destroyedMatch) {
        destroyedSet.add(destroyedMatch[1].trim());
      }
    }
    
    return {
      creating: creatingSet.size,
      created: createdSet.size,
      destroying: destroyingSet.size,
      destroyed: destroyedSet.size,
    };
  }, [deploymentStatus?.output]);

  // Memoized helper to parse Terraform outputs from the output string
  const parsedOutputs = useMemo(() => {
    if (!deploymentStatus?.output) return {};
    
    const outputs: Record<string, string> = {};
    const lines = deploymentStatus.output.split('\n');
    
    for (const line of lines) {
      // Match output lines like: workspace_url = "https://..."
      const match = line.match(/^(\w+)\s*=\s*"([^"]*)"$/);
      if (match) {
        outputs[match[1]] = match[2];
      }
    }
    
    return outputs;
  }, [deploymentStatus?.output]);

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch (e) {
      console.error("Failed to copy:", e);
    }
  };

  const openTemplateFolder = async () => {
    if (templatePath) {
      try {
        await invoke("open_folder", { path: templatePath });
      } catch (e) {
        console.error("Failed to open folder:", e);
      }
    }
  };

  const openDeploymentsFolder = async () => {
    try {
      const path = await invoke<string>("get_deployments_folder");
      await invoke("open_folder", { path });
    } catch (e) {
      console.error("Failed to open deployments folder:", e);
    }
  };

  const renderDeployment = () => {
    const status = deploymentStatus;
    
    // User-friendly step descriptions - change based on rollback state
    const stepInfo: Record<DeploymentStep, { title: string; description: string; icon: string }> = {
      ready: { title: "Ready to Deploy", description: "Click below to start the deployment process.", icon: "🚀" },
      initializing: { title: "Preparing Environment", description: "Setting up Terraform and downloading required providers...", icon: "⚙️" },
      planning: { title: "Analyzing Changes", description: "Determining what resources will be created...", icon: "📋" },
      review: { title: "Review & Confirm", description: "Review the planned changes and confirm to proceed.", icon: "✅" },
      deploying: isRollingBack 
        ? { title: "Cleaning Up Resources", description: "Removing deployed resources. This may take several minutes...", icon: "🧹" }
        : { title: "Creating Workspace", description: "Deploying your Databricks workspace. This may take 10-15 minutes...", icon: "🔨" },
      complete: isRollingBack
        ? { title: "Cleanup Complete!", description: "All resources have been successfully removed.", icon: "✅" }
        : { title: "Deployment Complete!", description: "Your Databricks workspace has been successfully created.", icon: "🎉" },
      failed: { title: "Deployment Failed", description: "An error occurred. Review the logs below for details.", icon: "❌" },
    };

    const currentStepInfo = stepInfo[deploymentStep];
    const isWorking = deploymentStep === "initializing" || deploymentStep === "planning" || deploymentStep === "deploying";

    return (
      <div className="container">
        {!isWorking && deploymentStep !== "complete" && (
          <button className="back-btn" onClick={goBack}>
            ← Back to Configuration
          </button>
        )}
        
        {/* Progress indicator */}
        <div className="wizard-progress">
          <div className={`wizard-step ${deploymentStep === "initializing" || deploymentStep === "planning" || deploymentStep === "review" || deploymentStep === "deploying" || deploymentStep === "complete" ? "active" : ""} ${deploymentStep === "failed" ? "failed" : ""}`}>
            <div className="wizard-step-number">1</div>
            <div className="wizard-step-label">Prepare</div>
          </div>
          <div className="wizard-connector" />
          <div className={`wizard-step ${deploymentStep === "review" || deploymentStep === "deploying" || deploymentStep === "complete" ? "active" : ""}`}>
            <div className="wizard-step-number">2</div>
            <div className="wizard-step-label">Review</div>
          </div>
          <div className="wizard-connector" />
          <div className={`wizard-step ${deploymentStep === "deploying" || deploymentStep === "complete" ? "active" : ""}`}>
            <div className="wizard-step-number">3</div>
            <div className="wizard-step-label">{isRollingBack ? "Cleanup" : "Deploy"}</div>
          </div>
        </div>

        <div className="deployment-status-card">
          {currentStepInfo.icon && <div className="status-icon">{currentStepInfo.icon}</div>}
          <h1>{currentStepInfo.title}</h1>
          <p className="subtitle">{currentStepInfo.description}</p>
          
          {isWorking && (
            <div className="progress-indicator">
              <span className="spinner large" />
              {deploymentStep === "deploying" && resourceCounts && (resourceCounts.creating > 0 || resourceCounts.destroying > 0) && (
                <div style={{ marginTop: "12px", fontSize: "14px", color: "#a6a6a6" }}>
                  {isRollingBack 
                    ? `${resourceCounts.destroyed} of ${resourceCounts.destroying} resources removed`
                    : `${resourceCounts.created} of ${resourceCounts.creating} resources created`
                  }
                </div>
              )}
            </div>
          )}
        </div>

        {error && <div className="alert alert-error">{error}</div>}

        {/* Show logs toggle and template folder - hide folder on complete since it's in summary */}
        <div className="status-controls" style={{ marginTop: "24px", justifyContent: deploymentStep === "complete" ? "center" : "space-between", alignItems: "center" }}>
          <label className="log-toggle">
            <input
              type="checkbox"
              checked={showDetailedLogs}
              onChange={(e) => setShowDetailedLogs(e.target.checked)}
            />
            Show detailed logs
          </label>
          {templatePath && deploymentStep !== "complete" && (
            <div style={{ fontSize: "12px", color: "#757575" }}>
              Deployment folder: <code style={{ marginRight: "8px" }}>{templatePath}</code>
              <button 
                onClick={openTemplateFolder}
                style={{ 
                  background: "none", 
                  border: "none", 
                  color: "#ff6b35", 
                  cursor: "pointer",
                  fontSize: "12px",
                  textDecoration: "underline"
                }}
              >
                Open
              </button>
            </div>
          )}
        </div>

        {/* Output */}
        {(showDetailedLogs || deploymentStep === "failed") && status?.output && (
          <div className={`output ${showDetailedLogs ? "expanded" : ""}`}>
            {status.output}
          </div>
        )}

        {/* Actions based on step */}
        <div style={{ marginTop: "24px", display: "flex", gap: "12px", justifyContent: "center" }}>
          {isWorking && (
            <button className="btn btn-danger" onClick={cancelDeployment}>
              Cancel
            </button>
          )}
          
          {deploymentStep === "review" && (
            <>
              <button className="btn btn-secondary" onClick={goBack}>
                Go Back & Edit
              </button>
              <button className="btn btn-large btn-success" onClick={confirmAndDeploy}>
                Confirm & Deploy →
              </button>
            </>
          )}
          
          {deploymentStep === "complete" && !isRollingBack && (() => {
            const workspaceUrl = parsedOutputs.workspace_url;
            const metastoreStatus = parsedOutputs.metastore_status;
            const metastoreId = parsedOutputs.metastore_id;
            
            return (
              <div style={{ 
                display: "flex", 
                flexDirection: "column", 
                alignItems: "center",
                width: "100%",
                maxWidth: "600px",
                margin: "0 auto"
              }}>
                {/* Workspace URL - Primary CTA */}
                {workspaceUrl && (
                  <div style={{
                    background: "linear-gradient(135deg, #1a3a1a 0%, #0d2610 100%)",
                    border: "1px solid #2d5a2d",
                    borderRadius: "12px",
                    padding: "24px",
                    marginBottom: "20px",
                    width: "100%",
                    textAlign: "center"
                  }}>
                    <div style={{ color: "#888", fontSize: "12px", marginBottom: "8px", textTransform: "uppercase", letterSpacing: "1px" }}>
                      Your Workspace
                    </div>
                    <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: "8px", marginBottom: "16px" }}>
                      <a 
                        href={workspaceUrl.startsWith("http") ? workspaceUrl : `https://${workspaceUrl}`} 
                        target="_blank" 
                        rel="noopener noreferrer"
                        style={{ 
                          color: "#4ade80", 
                          fontSize: "16px", 
                          fontWeight: "500",
                          wordBreak: "break-all"
                        }}
                      >
                        {workspaceUrl}
                      </a>
                      <button 
                        onClick={() => copyToClipboard(workspaceUrl)}
                        title="Copy URL"
                        style={{ 
                          background: "transparent", 
                          border: "none", 
                          color: "#4ade80",
                          cursor: "pointer",
                          fontSize: "14px",
                          padding: "4px"
                        }}
                      >
                        📋
                      </button>
                    </div>
                    <a 
                      href={workspaceUrl.startsWith("http") ? workspaceUrl : `https://${workspaceUrl}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="btn btn-success btn-large"
                      style={{ textDecoration: "none", display: "inline-block" }}
                    >
                      Open Workspace →
                    </a>
                  </div>
                )}

                {/* Unity Catalog Info */}
                {(metastoreStatus || metastoreId) && (
                  <div style={{
                    background: "#1e1e1e",
                    border: "1px solid #333",
                    borderRadius: "8px",
                    padding: "16px 20px",
                    marginBottom: "20px",
                    width: "100%"
                  }}>
                    <div style={{ 
                      display: "flex", 
                      alignItems: "center", 
                      gap: "12px",
                      flexWrap: "wrap"
                    }}>
                      <span style={{ fontSize: "20px" }}>📊</span>
                      <div style={{ flex: 1 }}>
                        <div style={{ color: "#e0e0e0", fontSize: "14px", fontWeight: "500" }}>
                          Unity Catalog
                        </div>
                        <div style={{ color: "#888", fontSize: "13px" }}>
                          {metastoreStatus || `Metastore: ${metastoreId}`}
                        </div>
                      </div>
                    </div>
                  </div>
                )}

                {/* Secondary Actions */}
                <div style={{ 
                  display: "flex", 
                  gap: "12px", 
                  width: "100%",
                  marginBottom: "20px"
                }}>
                  <button 
                    className="btn btn-secondary" 
                    style={{ flex: 1 }}
                    onClick={() => {
                      setScreen("welcome");
                      setSelectedCloud("");
                      setSelectedTemplate(null);
                      setDeploymentStep("ready");
                      clearSensitiveCredentials();
                    }}
                  >
                    New Deployment
                  </button>
                  {templatePath && (
                    <button 
                      className="btn btn-secondary" 
                      style={{ flex: 1 }}
                      onClick={openTemplateFolder}
                    >
                      Open Folder
                    </button>
                  )}
                </div>

                {/* Delete Option */}
                {status?.can_rollback && (
                  <button 
                    className="btn" 
                    onClick={rollback}
                    style={{ 
                      background: "transparent",
                      border: "1px solid #666",
                      color: "#888",
                      fontSize: "13px",
                      padding: "8px 16px"
                    }}
                  >
                    Delete Workspace & Resources
                  </button>
                )}
              </div>
            );
          })()}
          
          {deploymentStep === "complete" && isRollingBack && (
            <button className="btn" onClick={() => {
              setScreen("welcome");
              setSelectedCloud("");
              setSelectedTemplate(null);
              setDeploymentStep("ready");
              setIsRollingBack(false);
              clearSensitiveCredentials();
            }}>
              Start New Deployment
            </button>
          )}
          
          {deploymentStep === "failed" && (
            <>
              <button className="btn" onClick={() => startDeploymentWizard()}>
                Try Again
              </button>
              {status?.can_rollback && (
                <button className="btn btn-danger" onClick={rollback}>
                  Cleanup Resources
                </button>
              )}
            </>
          )}
        </div>
      </div>
    );
  };

  // Render based on current screen
  switch (screen) {
    case "welcome":
      return renderWelcome();
    case "dependencies":
      return renderDependencies();
    case "cloud-selection":
      return renderCloudSelection();
    case "databricks-credentials":
      return renderDatabricksCredentials();
    case "aws-credentials":
      return renderAwsCredentials();
    case "azure-credentials":
      return renderAzureCredentials();
    case "template-selection":
      return renderTemplateSelection();
    case "configuration":
      return renderConfiguration();
    case "deployment":
      return renderDeployment();
    default:
      return renderWelcome();
  }
}

// Helper functions
function groupVariablesBySection(
  variables: TerraformVariable[],
  _cloud: string
): Record<string, TerraformVariable[]> {
  const sections: Record<string, TerraformVariable[]> = {};
  
  // Define section mappings based on variable names
  const sectionMap: Record<string, string> = {
    prefix: "Workspace Configuration",
    resource_prefix: "Workspace Configuration",
    workspace_name: "Workspace Configuration",
    admin_user: "Workspace Configuration",
    root_storage_name: "Workspace Configuration",
    workspace_sku: "Workspace Configuration",
    pricing_tier: "Workspace Configuration",
    
    region: "Region & Tags",
    location: "Region & Tags",
    resource_group_name: "Region & Tags",
    tags: "Region & Tags",
    
    vpc_id: "Network Configuration",
    vpc_cidr_range: "Network Configuration",
    cidr_block: "Network Configuration",
    vnet_name: "Network Configuration",
    vnet_resource_group_name: "Network Configuration",
    cidr: "Network Configuration",
    availability_zones: "Network Configuration",
    subnet_ids: "Network Configuration",
    private_subnets_cidr: "Network Configuration",
    public_subnets_cidr: "Network Configuration",
    subnet_public_cidr: "Network Configuration",
    subnet_private_cidr: "Network Configuration",
    create_new_vnet: "Network Configuration",
    
    security_group_ids: "Security Configuration",
    sg_egress_ports: "Security Configuration",
    
    // Advanced: reuse existing resources (hidden by default)
    existing_vpc_id: "Advanced: Use Existing Resources",
    existing_subnet_ids: "Advanced: Use Existing Resources",
    existing_security_group_id: "Advanced: Use Existing Resources",
    metastore_id: "Advanced: Use Existing Resources",
    existing_metastore_id: "Advanced: Use Existing Resources",
  };

  variables.forEach((v) => {
    // Skip excluded variables (credentials collected in earlier screens)
    if ((EXCLUDE_VARIABLES as readonly string[]).includes(v.name)) {
      return;
    }
    
    const section = sectionMap[v.name] || "Other Configuration";
    if (!sections[section]) {
      sections[section] = [];
    }
    sections[section].push(v);
  });

  return sections;
}

function formatVariableName(name: string): string {
  // Use constant display names if available
  if (VARIABLE_DISPLAY_NAMES[name]) {
    return VARIABLE_DISPLAY_NAMES[name];
  }
  
  return name
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

export default App;
