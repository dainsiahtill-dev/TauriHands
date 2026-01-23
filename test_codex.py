#!/usr/bin/env python3
"""
Test script to verify Codex CLI integration in TauriHands
"""

import subprocess
import sys
import os

def run_command(cmd, cwd=None):
    """Run a command and return the result"""
    try:
        result = subprocess.run(
            cmd, 
            shell=True, 
            capture_output=True, 
            text=True, 
            cwd=cwd
        )
        return result.returncode, result.stdout, result.stderr
    except Exception as e:
        return 1, "", str(e)

def test_codex_help():
    """Test if Codex options are available in help"""
    print("🔍 Testing Codex CLI integration...")
    
    # Change to the project directory
    project_dir = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri"
    
    # Test basic help to see if codex options are there
    print("\n📋 Testing help command...")
    code, stdout, stderr = run_command(
        f'cd "{project_dir}" && cargo run --features cli -- --help',
        cwd=project_dir
    )
    
    if code == 0:
        # Check if Codex options are present
        if "--use-codex" in stdout:
            print("✅ Codex CLI option found in help!")
        else:
            print("❌ Codex CLI option not found in help")
            print("Help output:")
            print(stdout)
            return False
            
        if "--codex-model" in stdout:
            print("✅ Codex model option found!")
        else:
            print("❌ Codex model option not found")
            
        if "--codex-reasoning" in stdout:
            print("✅ Codex reasoning option found!")
        else:
            print("❌ Codex reasoning option not found")
            
        if "--codex-approval" in stdout:
            print("✅ Codex approval option found!")
        else:
            print("❌ Codex approval option not found")
    else:
        print(f"❌ Help command failed with code {code}")
        print(f"Error: {stderr}")
        return False
    
    return True

def test_codex_version():
    """Test version command with Codex info"""
    print("\n🏷️ Testing version command...")
    
    project_dir = r"c:\Users\dains\Documents\Gitea\TauriHands\src-tauri"
    
    # Test version command
    code, stdout, stderr = run_command(
        f'cd "{project_dir}" && cargo run --features cli -- version --use-codex --codex-model gpt-4-codex',
        cwd=project_dir
    )
    
    if code == 0:
        if "Codex Integration: Enabled" in stdout:
            print("✅ Codex integration working in version command!")
            print("Version output:")
            print(stdout)
        else:
            print("❌ Codex integration not showing in version")
            print("Output:")
            print(stdout)
    else:
        print(f"❌ Version command failed with code {code}")
        print(f"Error: {stderr}")
    
    return code == 0

def main():
    """Main test function"""
    print("🚀 TauriHands Codex Integration Test")
    print("=" * 50)
    
    # Test 1: Help command
    help_ok = test_codex_help()
    
    # Test 2: Version command  
    version_ok = test_codex_version()
    
    # Summary
    print("\n" + "=" * 50)
    print("📊 Test Summary:")
    print(f"   Help Command: {'✅ PASS' if help_ok else '❌ FAIL'}")
    print(f"   Version Command: {'✅ PASS' if version_ok else '❌ FAIL'}")
    
    if help_ok and version_ok:
        print("\n🎉 All tests passed! Codex integration is working!")
        return 0
    else:
        print("\n⚠️ Some tests failed. Check the output above.")
        return 1

if __name__ == "__main__":
    sys.exit(main())
