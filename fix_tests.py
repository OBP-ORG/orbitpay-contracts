import re

file_path = "contracts/payroll_stream/src/test.rs"
with open(file_path, "r") as f:
    content = f.read()

duplicate_setup = """    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token_client = create_token_client(&env, &token_contract.address);
    token_contract.mint(&sender, &10000);"""
content = content.replace(duplicate_setup, "")

duplicate_setup2 = """    let token_contract = create_token_contract(&env, &token_admin);
    let token_client = create_token_client(&env, &token_contract.address);
    token_contract.mint(&sender, &10000);"""
content = content.replace(duplicate_setup2, "")

duplicate_create = """    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_contract.address,
        &10000_i128,
        &1000_u64,
        &2000_u64,
    );"""
content = content.replace(duplicate_create, "")

# Duplicate assertions in test_create_stream_fails_without_balance_and_does_not_persist
dup_asserts = """    
    assert_eq!(token_client.balance(&sender), 0);
    assert_eq!(token_client.balance(&client.address), 10000);"""
content = content.replace(dup_asserts, "")

with open(file_path, "w") as f:
    f.write(content)
