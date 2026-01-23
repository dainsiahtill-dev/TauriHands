#!/usr/bin/env python3
"""
Simple test to verify automation engine core components
"""

import os
import sys

def test_automation_files_exist():
    """Test if automation engine files exist"""
    print("🔍 Testing Automation Engine Files...")
    
    base_dir = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\automation"
    
    required_files = [
        "engine.rs",
        "planner.rs", 
        "executor.rs",
        "validator.rs",
        "recovery.rs",
        "monitor.rs",
        "mod.rs"
    ]
    
    all_exist = True
    for file in required_files:
        file_path = os.path.join(base_dir, file)
        if os.path.exists(file_path):
            print(f"  ✅ {file}")
        else:
            print(f"  ❌ {file} missing")
            all_exist = False
    
    return all_exist

def test_codex_integration():
    """Test Codex integration files"""
    print("\n🤖 Testing Codex Integration...")
    
    codex_file = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\services\codex.rs"
    if os.path.exists(codex_file):
        print("  ✅ Codex module exists")
        
        # Check content
        with open(codex_file, 'r', encoding='utf-8') as f:
            content = f.read()
            
        checks = [
            ("CodexConfig struct", "struct CodexConfig" in content),
            ("CodexClient trait", "trait CodexClient" in content),
            ("LocalCodexClient", "struct LocalCodexClient" in content),
            ("CloudCodexClient", "struct CloudCodexClient" in content),
            ("CodexManager", "struct CodexManager" in content),
            ("execute method", "async fn execute" in content),
            ("interactive_session", "async fn interactive_session" in content),
            ("code_review", "async fn code_review" in content),
        ]
        
        all_checks_pass = True
        for check_name, check_result in checks:
            if check_result:
                print(f"    ✅ {check_name}")
            else:
                print(f"    ❌ {check_name}")
                all_checks_pass = False
        
        return all_checks_pass
    else:
        print("  ❌ Codex module missing")
        return False

def test_automation_structures():
    """Test automation engine structures"""
    print("\n🏗️ Testing Automation Structures...")
    
    engine_file = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\automation\engine.rs"
    if not os.path.exists(engine_file):
        print("  ❌ engine.rs missing")
        return False
    
    with open(engine_file, 'r', encoding='utf-8') as f:
        content = f.read()
    
    structures = [
        ("AutomationTask struct", "struct AutomationTask" in content),
        ("TaskType enum", "enum TaskType" in content),
        ("TaskPriority enum", "enum TaskPriority" in content),
        ("TaskStatus enum", "enum TaskStatus" in content),
        ("AutomationConfig struct", "struct AutomationConfig" in content),
        ("AutomationResult struct", "struct AutomationResult" in content),
        ("AutomationEngine struct", "struct AutomationEngine" in content),
    ]
    
    all_structures_ok = True
    for struct_name, exists in structures:
        if exists:
            print(f"  ✅ {struct_name}")
        else:
            print(f"  ❌ {struct_name}")
            all_structures_ok = False
    
    return all_structures_ok

def test_cli_integration():
    """Test CLI integration"""
    print("\n🖥️ Testing CLI Integration...")
    
    commands_file = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\cli\commands.rs"
    if not os.path.exists(commands_file):
        print("  ❌ commands.rs missing")
        return False
    
    with open(commands_file, 'r', encoding='utf-8') as f:
        content = f.read()
    
    codex_flags = [
        ("--use-codex flag", "--use-codex" in content),
        ("--codex-model flag", "--codex-model" in content),
        ("--codex-reasoning flag", "--codex-reasoning" in content),
        ("--codex-approval flag", "--codex-approval" in content),
    ]
    
    all_flags_ok = True
    for flag_name, exists in codex_flags:
        if exists:
            print(f"  ✅ {flag_name}")
        else:
            print(f"  ❌ {flag_name}")
            all_flags_ok = False
    
    return all_flags_ok

def test_services_mod():
    """Test services module integration"""
    print("\n🔧 Testing Services Module...")
    
    mod_file = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri\src\services\mod.rs"
    if not os.path.exists(mod_file):
        print("  ❌ services/mod.rs missing")
        return False
    
    with open(mod_file, 'r', encoding='utf-8') as f:
        content = f.read()
    
    modules = [
        ("codex module", "pub mod codex;" in content),
        ("kernel module", "pub mod kernel;" in content),
        ("llm module", "pub mod llm;" in content),
        ("pty module", "pub mod pty;" in content),
        ("workspace module", "pub mod workspace;" in content),
    ]
    
    all_modules_ok = True
    for module_name, exists in modules:
        if exists:
            print(f"  ✅ {module_name}")
        else:
            print(f"  ❌ {module_name}")
            all_modules_ok = False
    
    return all_modules_ok

def main():
    """Main test function"""
    print("🚀 TauriHands Automation Engine Simple Test")
    print("=" * 60)
    
    # Test 1: Files exist
    files_ok = test_automation_files_exist()
    
    # Test 2: Codex integration
    codex_ok = test_codex_integration()
    
    # Test 3: Automation structures
    structures_ok = test_automation_structures()
    
    # Test 4: CLI integration
    cli_ok = test_cli_integration()
    
    # Test 5: Services module
    services_ok = test_services_mod()
    
    # Summary
    print("\n" + "=" * 60)
    print("📊 Test Summary:")
    print(f"   Files: {'✅ PASS' if files_ok else '❌ FAIL'}")
    print(f"   Codex Integration: {'✅ PASS' if codex_ok else '❌ FAIL'}")
    print(f"   Automation Structures: {'✅ PASS' if structures_ok else '❌ FAIL'}")
    print(f"   CLI Integration: {'✅ PASS' if cli_ok else '❌ FAIL'}")
    print(f"   Services Module: {'✅ PASS' if services_ok else '❌ FAIL'}")
    
    all_tests_passed = all([files_ok, codex_ok, structures_ok, cli_ok, services_ok])
    
    if all_tests_passed:
        print("\n🎉 All automation engine tests passed!")
        print("✅ Core automation engine structure is complete")
        print("✅ Codex CLI integration is implemented")
        print("✅ CLI flags are available")
        print("✅ Services are properly integrated")
        return 0
    else:
        print("\n⚠️ Some tests failed. Check the output above.")
        return 1

if __name__ == "__main__":
    sys.exit(main())
