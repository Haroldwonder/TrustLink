#!/usr/bin/env node

/**
 * Pre-publish validation script for @trustlink/sdk
 * 
 * This script validates the package structure and contents before publishing to npm.
 * Run with: node scripts/validate-package.js
 */

const fs = require('fs');
const path = require('path');

const REQUIRED_FILES = [
  'package.json',
  'README.md',
  'LICENSE',
  'dist/index.js',
  'dist/index.d.ts'
];

const REQUIRED_PACKAGE_FIELDS = [
  'name',
  'version',
  'description',
  'main',
  'types',
  'license',
  'repository',
  'homepage'
];

function validatePackage() {
  console.log('🔍 Validating @trustlink/sdk package...');
  
  let errors = 0;
  
  // Check required files exist
  console.log('\n📁 Checking required files...');
  for (const file of REQUIRED_FILES) {
    if (fs.existsSync(file)) {
      console.log(`  ✅ ${file}`);
    } else {
      console.log(`  ❌ ${file} - MISSING`);
      errors++;
    }
  }
  
  // Validate package.json
  console.log('\n📦 Validating package.json...');
  try {
    const pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
    
    for (const field of REQUIRED_PACKAGE_FIELDS) {
      if (pkg[field]) {
        console.log(`  ✅ ${field}: ${JSON.stringify(pkg[field])}`);
      } else {
        console.log(`  ❌ ${field} - MISSING`);
        errors++;
      }
    }
    
    // Check version format
    const versionRegex = /^\d+\.\d+\.\d+(-[a-zA-Z0-9.-]+)?$/;
    if (!versionRegex.test(pkg.version)) {
      console.log(`  ❌ version format invalid: ${pkg.version}`);
      errors++;
    }
    
    // Check files array
    if (!pkg.files || !Array.isArray(pkg.files)) {
      console.log('  ❌ files array missing or invalid');
      errors++;
    } else if (!pkg.files.includes('dist')) {
      console.log('  ❌ files array must include "dist"');
      errors++;
    }
    
  } catch (err) {
    console.log(`  ❌ Failed to parse package.json: ${err.message}`);
    errors++;
  }
  
  // Check TypeScript build output
  console.log('\n🔧 Checking TypeScript build...');
  try {
    const indexJs = fs.readFileSync('dist/index.js', 'utf8');
    const indexDts = fs.readFileSync('dist/index.d.ts', 'utf8');
    
    if (indexJs.includes('TrustLinkClient')) {
      console.log('  ✅ index.js contains TrustLinkClient');
    } else {
      console.log('  ❌ index.js missing TrustLinkClient export');
      errors++;
    }
    
    if (indexDts.includes('TrustLinkClient')) {
      console.log('  ✅ index.d.ts contains TrustLinkClient types');
    } else {
      console.log('  ❌ index.d.ts missing TrustLinkClient types');
      errors++;
    }
    
  } catch (err) {
    console.log(`  ❌ Failed to read build output: ${err.message}`);
    errors++;
  }
  
  // Check package size
  console.log('\n📏 Checking package size...');
  try {
    const { execSync } = require('child_process');
    const packOutput = execSync('npm pack --dry-run --silent', { encoding: 'utf8' });
    const lines = packOutput.trim().split('\n');
    const sizeLine = lines.find(line => line.includes('package size:'));
    
    if (sizeLine) {
      console.log(`  ℹ️  ${sizeLine}`);
      
      // Extract size in bytes
      const sizeMatch = sizeLine.match(/(\d+)\s*B/);
      if (sizeMatch) {
        const sizeBytes = parseInt(sizeMatch[1]);
        const sizeMB = sizeBytes / (1024 * 1024);
        
        if (sizeMB > 5) {
          console.log(`  ⚠️  Package size is large: ${sizeMB.toFixed(2)}MB`);
        } else {
          console.log(`  ✅ Package size is reasonable: ${sizeMB.toFixed(2)}MB`);
        }
      }
    }
  } catch (err) {
    console.log(`  ⚠️  Could not check package size: ${err.message}`);
  }
  
  // Summary
  console.log('\n' + '='.repeat(50));
  if (errors === 0) {
    console.log('🎉 Package validation PASSED! Ready to publish.');
    process.exit(0);
  } else {
    console.log(`❌ Package validation FAILED with ${errors} error(s).`);
    console.log('Please fix the issues above before publishing.');
    process.exit(1);
  }
}

// Run validation
validatePackage();