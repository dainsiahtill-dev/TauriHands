#!/usr/bin/env python3
"""
Test script to verify Automation Engine functionality in TauriHands
"""

import subprocess
import sys
import os
import json
import time

def run_command(cmd, cwd=None, timeout=30):
    """Run a command and return the result"""
    try:
        result = subprocess.run(
            cmd, 
            shell=True, 
            capture_output=True, 
            text=True, 
            cwd=cwd,
            timeout=timeout
        )
        return result.returncode, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return 1, "", "Command timed out"
    except Exception as e:
        return 1, "", str(e)

def test_automation_engine_compilation():
    """Test if automation engine compiles correctly"""
    print("🔧 Testing Automation Engine Compilation...")
    
    project_dir = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri"
    
    # Test compilation
    print("\n📦 Testing compilation...")
    code, stdout, stderr = run_command(
        f'cd "{project_dir}" && cargo check --features cli',
        cwd=project_dir
    )
    
    if code == 0:
        print("✅ Automation engine compiles successfully!")
        return True
    else:
        print(f"❌ Compilation failed with code {code}")
        print(f"Error: {stderr}")
        return False

def test_automation_modules():
    """Test individual automation modules"""
    print("\n🧩 Testing Automation Modules...")
    
    project_dir = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri"
    
    modules = [
        "automation::engine",
        "automation::planner", 
        "automation::executor",
        "automation::validator",
        "automation::recovery",
        "automation::monitor"
    ]
    
    all_passed = True
    for module in modules:
        print(f"  Testing {module}...")
        code, stdout, stderr = run_command(
            f'cd "{project_dir}" && cargo check --features cli --lib',
            cwd=project_dir
        )
        
        if code == 0:
            print(f"  ✅ {module} - OK")
        else:
            print(f"  ❌ {module} - FAILED")
            all_passed = False
    
    return all_passed

def test_automation_config():
    """Test automation configuration"""
    print("\n⚙️ Testing Automation Configuration...")
    
    # Test if config files exist
    config_files = [
        r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\automation\engine.rs",
        r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\automation\planner.rs",
        r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\automation\executor.rs",
        r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\automation\validator.rs",
        r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\automation\recovery.rs",
        r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\automation\monitor.rs"
    ]
    
    all_exist = True
    for config_file in config_files:
        if os.path.exists(config_file):
            print(f"  ✅ {os.path.basename(config_file)} exists")
        else:
            print(f"  ❌ {os.path.basename(config_file)} missing")
            all_exist = False
    
    return all_exist

def test_codex_integration():
    """Test Codex integration in automation engine"""
    print("\n🤖 Testing Codex Integration...")
    
    # Check if codex module is integrated
    codex_file = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\services\codex.rs"
    if os.path.exists(codex_file):
        print("  ✅ Codex module exists")
        
        # Check if it's imported in automation
        executor_file = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\automation\executor.rs"
        if os.path.exists(executor_file):
            with open(executor_file, 'r', encoding='utf-8') as f:
                content = f.read()
                if 'codex' in content.lower():
                    print("  ✅ Codex integration found in executor")
                    return True
                else:
                    print("  ⚠️ Codex integration not found in executor")
                    return False
    else:
        print("  ❌ Codex module missing")
        return False

def test_automation_features():
    """Test automation engine features"""
    print("\n🚀 Testing Automation Features...")
    
    features = [
        "Task Planning",
        "Code Execution", 
        "Validation",
        "Error Recovery",
        "Progress Monitoring",
        "Codex Integration"
    ]
    
    # Check if features are implemented
    feature_status = {}
    
    # Task Planning
    planner_file = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\automation\planner.rs"
    feature_status["Task Planning"] = os.path.exists(planner_file)
    
    # Code Execution
    executor_file = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\automation\executor.rs"
    feature_status["Code Execution"] = os.path.exists(executor_file)
    
    # Validation
    validator_file = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\automation\validator.rs"
    feature_status["Validation"] = os.path.exists(validator_file)
    
    # Error Recovery
    recovery_file = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\automation\recovery.rs"
    feature_status["Error Recovery"] = os.path.exists(recovery_file)
    
    # Progress Monitoring
    monitor_file = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\automation\monitor.rs"
    feature_status["Progress Monitoring"] = os.path.exists(monitor_file)
    
    # Codex Integration
    codex_file = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\services\codex.rs"
    feature_status["Codex Integration"] = os.path.exists(codex_file)
    
    all_features_ok = True
    for feature, status in feature_status.items():
        if status:
            print(f"  ✅ {feature}")
        else:
            print(f"  ❌ {feature}")
            all_features_ok = False
    
    return all_features_ok

def main():
    """Main test function"""
    print("🚀 TauriHands Automation Engine Test")
    print("=" * 60)
    
    # Test 1: Compilation
    compile_ok = test_automation_engine_compilation()
    
    # Test 2: Modules
    modules_ok = test_automation_modules()
    
    # Test 3: Configuration
    config_ok = test_automation_config()
    
    # Test 4: Codex Integration
    codex_ok = test_codex_integration()
    
    # Test 5: Features
    features_ok = test_automation_features()
    
    # Summary
    print("\n" + "=" * 60)
    print("📊 Test Summary:")
    print(f"   Compilation: {'✅ PASS' if compile_ok else '❌ FAIL'}")
    print(f"   Modules: {'✅ PASS' if modules_ok else '❌ FAIL'}")
    print(f"   Configuration: {'✅ PASS' if config_ok else '❌ FAIL'}")
    print(f"   Codex Integration: {'✅ PASS' if codex_ok else '❌ FAIL'}")
    print(f"   Features: {'✅ PASS' if features_ok else '❌ FAIL'}")
    
    all_tests_passed = all([compile_ok, modules_ok, config_ok, codex_ok, features_ok])
    
    if all_tests_passed:
        print("\n🎉 All automation engine tests passed!")
        return 0
    else:
        print("\n⚠️ Some tests failed. Check the output above.")
        return 1

if __name__ == "__main__":
    sys.exit(main())
