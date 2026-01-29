# Template Folder Structure Reorganization

## Summary

Reorganized template folders to improve user experience when downloading templates. Users now see a simple README.txt first, with all technical files in a subfolder.

## New Structure

### Before:
```
downloaded-template/
├── deploy.py
├── *.tf files...
├── README.md
├── README.txt
├── requirements.txt
└── ...
```
**Problem**: Too many files, overwhelming for users

### After:
```
downloaded-template/
├── README.txt (Simple 3-step quick start guide)
└── terraform/
    ├── deploy.py
    ├── *.tf files
    ├── README.md (Detailed documentation)
    ├── requirements.txt
    └── terraform.tfvars
```
**Benefit**: Clean, user-friendly - README.txt guides them to the terraform folder

## Changes Made

### 1. Folder Structure (Both Azure & AWS)
- Created `terraform/` subfolder in each template
- Moved all technical files into `terraform/`:
  - All `.tf` files
  - `deploy.py`
  - `requirements.txt`
  - `README.md`
  - `terraform.tfvars`
  - `.gitignore` (AWS only)
- Kept `README.txt` at root with simple 3-step instructions

### 2. Updated README.txt Files
Both templates now have user-friendly README.txt with:
- Clear 3-step quick start guide
- Instructions to navigate to terraform folder
- Port information (8080)
- Reference to detailed documentation

### 3. Updated Code
**app/models/template.py**:
- Added `directory` property (alias for `path`)
- Updated `tfvars_path` to point to `terraform/terraform.tfvars`
- Updated `get_available_templates()` to check terraform subfolder

## User Experience Flow

1. User downloads template from Flask app
2. Opens ZIP file and sees:
   ```
   README.txt  <-- Clear, simple file at the top
   terraform/  <-- All the technical stuff
   ```
3. Opens README.txt and reads:
   - Step 1: `cd terraform`
   - Step 2: Install dependencies
   - Step 3: Run `python deploy.py`
4. Follows 3 simple steps to deploy!

## Testing

Verified template detection works correctly:
```
Found 2 templates:
  - azure-simple: templates/azure-simple/terraform/terraform.tfvars
  - aws-simple: templates/aws-simple/terraform/terraform.tfvars
```

## Benefits

✅ **Cleaner first impression** - Single README.txt instead of dozens of files  
✅ **Better UX** - Clear 3-step quick start guide  
✅ **Less overwhelming** - Technical files hidden in subfolder  
✅ **Professional** - Matches best practices for downloadable packages  
✅ **Scalable** - Easy to add more templates with same structure

## Compatibility

- ✅ Flask app detects templates correctly
- ✅ Download functionality maintains folder structure
- ✅ Deploy.py scripts work from terraform subfolder
- ✅ All paths updated correctly
