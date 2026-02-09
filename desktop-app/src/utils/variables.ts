import { TerraformVariable } from "../types";
import { EXCLUDE_VARIABLES, VARIABLE_DISPLAY_NAMES, DEFAULTS } from "../constants";

/**
 * Canonical section order for the configuration form.
 */
export const SECTION_ORDER = ["Workspace", "Advanced: Network Configuration", "Tags"];

/**
 * Field display order within each section (lower number = displayed first).
 */
const FIELD_ORDER: Record<string, number> = {
  // Workspace - consistent order across clouds: name, region, admin, then cloud-specific extras
  prefix: 1,
  workspace_name: 1,
  databricks_workspace_name: 1,
  region: 2,
  location: 2,
  google_region: 2,
  admin_user: 3,
  resource_group_name: 4,
  root_storage_name: 5,
  workspace_sku: 6,
  // Tags
  tags: 1,
  // Advanced: Network - toggle first, CIDRs, then existing resources
  create_new_vnet: 1,
  cidr_block: 2,
  cidr: 2,
  subnet_cidr: 2,
  subnet_public_cidr: 3,
  subnet_private_cidr: 4,
  vnet_name: 5,
  vnet_resource_group_name: 6,
  existing_vpc_id: 20,
  existing_subnet_ids: 21,
  existing_security_group_id: 22,
};

/**
 * Groups Terraform variables by section for display in the configuration form.
 * Returns sections in canonical order with fields sorted within each section.
 */
export function groupVariablesBySection(
  variables: TerraformVariable[]
): Record<string, TerraformVariable[]> {
  const sectionMap: Record<string, string> = {
    // Workspace
    prefix: "Workspace",
    workspace_name: "Workspace",
    databricks_workspace_name: "Workspace",
    admin_user: "Workspace",
    root_storage_name: "Workspace",
    workspace_sku: "Workspace",
    region: "Workspace",
    location: "Workspace",
    google_region: "Workspace",
    resource_group_name: "Workspace",

    // Tags
    tags: "Tags",

    // Advanced: Network Configuration (all network + existing resource fields)
    vpc_id: "Advanced: Network Configuration",
    vpc_cidr_range: "Advanced: Network Configuration",
    cidr_block: "Advanced: Network Configuration",
    vnet_name: "Advanced: Network Configuration",
    vnet_resource_group_name: "Advanced: Network Configuration",
    cidr: "Advanced: Network Configuration",
    availability_zones: "Advanced: Network Configuration",
    subnet_ids: "Advanced: Network Configuration",
    private_subnets_cidr: "Advanced: Network Configuration",
    public_subnets_cidr: "Advanced: Network Configuration",
    subnet_public_cidr: "Advanced: Network Configuration",
    subnet_private_cidr: "Advanced: Network Configuration",
    subnet_cidr: "Advanced: Network Configuration",
    create_new_vnet: "Advanced: Network Configuration",
    security_group_ids: "Advanced: Network Configuration",
    sg_egress_ports: "Advanced: Network Configuration",
    existing_vpc_id: "Advanced: Network Configuration",
    existing_subnet_ids: "Advanced: Network Configuration",
    existing_security_group_id: "Advanced: Network Configuration",
  };

  // Build temporary map
  const tempSections: Record<string, TerraformVariable[]> = {};
  variables.forEach((v) => {
    if ((EXCLUDE_VARIABLES as readonly string[]).includes(v.name)) return;
    const section = sectionMap[v.name] || "Other Configuration";
    if (!tempSections[section]) tempSections[section] = [];
    tempSections[section].push(v);
  });

  // Return sections in canonical order with fields sorted
  const sections: Record<string, TerraformVariable[]> = {};
  for (const name of SECTION_ORDER) {
    if (tempSections[name]) {
      sections[name] = tempSections[name].sort(
        (a, b) => (FIELD_ORDER[a.name] ?? 99) - (FIELD_ORDER[b.name] ?? 99)
      );
    }
  }
  // Include any remaining sections not in SECTION_ORDER
  for (const [name, vars] of Object.entries(tempSections)) {
    if (!sections[name]) {
      sections[name] = vars.sort(
        (a, b) => (FIELD_ORDER[a.name] ?? 99) - (FIELD_ORDER[b.name] ?? 99)
      );
    }
  }
  return sections;
}

/**
 * Formats a variable name for display (converts snake_case to Title Case)
 */
export function formatVariableName(name: string): string {
  // Use constant display names if available
  if (VARIABLE_DISPLAY_NAMES[name]) {
    return VARIABLE_DISPLAY_NAMES[name];
  }
  
  return name
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

/**
 * Generates a random suffix for resource names
 */
export function generateRandomSuffix(): string {
  return Math.random().toString(36).substring(2, 8);
}

interface FormDefaultsContext {
  azureUser?: string | null;
  gcpAccount?: string | null;
}

/**
 * Initializes form values with smart defaults for a set of Terraform variables.
 * Generates random suffixes for resource names and applies cloud-specific defaults.
 */
export function initializeFormDefaults(
  variables: TerraformVariable[],
  context: FormDefaultsContext = {}
): Record<string, any> {
  const defaults: Record<string, any> = {};
  const randomSuffix = generateRandomSuffix();
  const shortSuffix = randomSuffix.replace(/-/g, "").slice(0, DEFAULTS.SUFFIX_LENGTH);

  variables.forEach((v) => {
    if (v.name === "prefix") {
      const basePrefix = v.default || "databricks";
      defaults[v.name] = `${basePrefix}-${randomSuffix}`;
    } else if (v.name === "workspace_name" || v.name === "databricks_workspace_name") {
      defaults[v.name] = `databricks-ws-${randomSuffix}`;
    } else if (v.name === "root_storage_name") {
      defaults[v.name] = `dbstorage${shortSuffix}`;
    } else if (v.name === "subnet_public_cidr") {
      defaults[v.name] = DEFAULTS.PUBLIC_SUBNET_CIDR;
    } else if (v.name === "subnet_private_cidr") {
      defaults[v.name] = DEFAULTS.PRIVATE_SUBNET_CIDR;
    } else if (v.name === "location") {
      defaults[v.name] = DEFAULTS.AZURE_REGION;
    } else if (v.name === "google_region") {
      defaults[v.name] = v.default || DEFAULTS.GCP_REGION;
    } else if (v.name === "admin_user" && context.azureUser) {
      defaults[v.name] = context.azureUser;
    } else if (v.name === "admin_user" && context.gcpAccount) {
      defaults[v.name] = context.gcpAccount;
    } else if (v.name === "create_new_resource_group") {
      defaults[v.name] = true;
    } else if (v.name === "vnet_name" || v.name === "vnet_resource_group_name") {
      // Leave empty - only filled when using existing VNet
      defaults[v.name] = "";
    } else if (v.default !== null) {
      defaults[v.name] = v.default;
    } else {
      defaults[v.name] = "";
    }
  });

  return defaults;
}
