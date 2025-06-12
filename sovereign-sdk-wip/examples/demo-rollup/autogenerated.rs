pub const CHAIN_HASH: [u8; 32] = [252, 225, 218, 69, 210, 191, 94, 218, 212, 200, 46, 182, 119, 118, 238, 200, 103, 203, 46, 12, 6, 78, 86, 211, 13, 205, 194, 202, 162, 106, 30, 41];

#[allow(dead_code)]
pub const SCHEMA_JSON: &str = r#"{
  "types": [
    {
      "Struct": {
        "type_name": "Transaction",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "versioned_tx",
            "silent": false,
            "value": {
              "ByIndex": 1
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Enum": {
        "type_name": "VersionedTx",
        "variants": [
          {
            "name": "V0",
            "discriminant": 0,
            "template": null,
            "value": {
              "ByIndex": 2
            }
          }
        ],
        "hide_tag": false
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 3
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "Version0",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "signature",
            "silent": false,
            "value": {
              "ByIndex": 4
            },
            "doc": ""
          },
          {
            "display_name": "pub_key",
            "silent": false,
            "value": {
              "ByIndex": 5
            },
            "doc": ""
          },
          {
            "display_name": "runtime_call",
            "silent": false,
            "value": {
              "ByIndex": 6
            },
            "doc": ""
          },
          {
            "display_name": "generation",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          },
          {
            "display_name": "details",
            "silent": false,
            "value": {
              "ByIndex": 108
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "Ed25519Signature",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "msg_sig",
            "silent": false,
            "value": {
              "Immediate": {
                "ByteArray": {
                  "len": 64,
                  "display": "Hex"
                }
              }
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "Ed25519PublicKey",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "pub_key",
            "silent": false,
            "value": {
              "Immediate": {
                "ByteArray": {
                  "len": 32,
                  "display": "Hex"
                }
              }
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Enum": {
        "type_name": "RuntimeCall",
        "variants": [
          {
            "name": "Bank",
            "discriminant": 0,
            "template": null,
            "value": {
              "ByIndex": 7
            }
          },
          {
            "name": "SequencerRegistry",
            "discriminant": 1,
            "template": null,
            "value": {
              "ByIndex": 25
            }
          },
          {
            "name": "ValueSetter",
            "discriminant": 2,
            "template": null,
            "value": {
              "ByIndex": 32
            }
          },
          {
            "name": "AttesterIncentives",
            "discriminant": 3,
            "template": null,
            "value": {
              "ByIndex": 40
            }
          },
          {
            "name": "ProverIncentives",
            "discriminant": 4,
            "template": null,
            "value": {
              "ByIndex": 45
            }
          },
          {
            "name": "Accounts",
            "discriminant": 5,
            "template": null,
            "value": {
              "ByIndex": 49
            }
          },
          {
            "name": "Uniqueness",
            "discriminant": 6,
            "template": null,
            "value": {
              "ByIndex": 54
            }
          },
          {
            "name": "ChainState",
            "discriminant": 7,
            "template": null,
            "value": {
              "ByIndex": 56
            }
          },
          {
            "name": "BlobStorage",
            "discriminant": 8,
            "template": null,
            "value": {
              "ByIndex": 57
            }
          },
          {
            "name": "Paymaster",
            "discriminant": 9,
            "template": null,
            "value": {
              "ByIndex": 58
            }
          },
          {
            "name": "Evm",
            "discriminant": 10,
            "template": null,
            "value": {
              "ByIndex": 84
            }
          },
          {
            "name": "AccessPattern",
            "discriminant": 11,
            "template": null,
            "value": {
              "ByIndex": 87
            }
          }
        ],
        "hide_tag": false
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 8
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Enum": {
        "type_name": "CallMessage",
        "variants": [
          {
            "name": "CreateToken",
            "discriminant": 0,
            "template": null,
            "value": {
              "ByIndex": 9
            }
          },
          {
            "name": "Transfer",
            "discriminant": 1,
            "template": "Transfer to address {} {}.",
            "value": {
              "ByIndex": 19
            }
          },
          {
            "name": "Burn",
            "discriminant": 2,
            "template": null,
            "value": {
              "ByIndex": 22
            }
          },
          {
            "name": "Mint",
            "discriminant": 3,
            "template": null,
            "value": {
              "ByIndex": 23
            }
          },
          {
            "name": "Freeze",
            "discriminant": 4,
            "template": null,
            "value": {
              "ByIndex": 24
            }
          }
        ],
        "hide_tag": false
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_CallMessage_CreateToken",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "token_name",
            "silent": false,
            "value": {
              "Immediate": "String"
            },
            "doc": ""
          },
          {
            "display_name": "token_decimals",
            "silent": false,
            "value": {
              "ByIndex": 10
            },
            "doc": ""
          },
          {
            "display_name": "initial_balance",
            "silent": false,
            "value": {
              "ByIndex": 11
            },
            "doc": ""
          },
          {
            "display_name": "mint_to_address",
            "silent": false,
            "value": {
              "ByIndex": 12
            },
            "doc": ""
          },
          {
            "display_name": "admins",
            "silent": false,
            "value": {
              "ByIndex": 17
            },
            "doc": ""
          },
          {
            "display_name": "supply_cap",
            "silent": false,
            "value": {
              "ByIndex": 18
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Option": {
        "value": {
          "Immediate": {
            "Integer": [
              "u8",
              "Decimal"
            ]
          }
        }
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "Immediate": {
                "Integer": [
                  "u128",
                  "Decimal"
                ]
              }
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Enum": {
        "type_name": "MultiAddress",
        "variants": [
          {
            "name": "Standard",
            "discriminant": 0,
            "template": null,
            "value": {
              "ByIndex": 13
            }
          },
          {
            "name": "Vm",
            "discriminant": 1,
            "template": null,
            "value": {
              "ByIndex": 15
            }
          }
        ],
        "hide_tag": true
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 14
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "Immediate": {
                "ByteArray": {
                  "len": 28,
                  "display": {
                    "Bech32m": {
                      "prefix": "sov"
                    }
                  }
                }
              }
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 16
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "Immediate": {
                "ByteArray": {
                  "len": 20,
                  "display": "Hex"
                }
              }
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Vec": {
        "value": {
          "ByIndex": 12
        }
      }
    },
    {
      "Option": {
        "value": {
          "ByIndex": 11
        }
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_CallMessage_Transfer",
        "template": "Transfer to address {} {}.",
        "peekable": false,
        "fields": [
          {
            "display_name": "to",
            "silent": false,
            "value": {
              "ByIndex": 12
            },
            "doc": ""
          },
          {
            "display_name": "coins",
            "silent": false,
            "value": {
              "ByIndex": 20
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "Coins",
        "template": "{} coins of token ID {}",
        "peekable": true,
        "fields": [
          {
            "display_name": "amount",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u128",
                  {
                    "FixedPoint": {
                      "FromSiblingField": {
                        "field_index": 1,
                        "byte_offset": 31
                      }
                    }
                  }
                ]
              }
            },
            "doc": ""
          },
          {
            "display_name": "token_id",
            "silent": false,
            "value": {
              "ByIndex": 21
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "Immediate": {
                "ByteArray": {
                  "len": 32,
                  "display": {
                    "Bech32m": {
                      "prefix": "token_"
                    }
                  }
                }
              }
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_CallMessage_Burn",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "coins",
            "silent": false,
            "value": {
              "ByIndex": 20
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_CallMessage_Mint",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "coins",
            "silent": false,
            "value": {
              "ByIndex": 20
            },
            "doc": ""
          },
          {
            "display_name": "mint_to_address",
            "silent": false,
            "value": {
              "ByIndex": 12
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_CallMessage_Freeze",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "token_id",
            "silent": false,
            "value": {
              "ByIndex": 21
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 26
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Enum": {
        "type_name": "CallMessage",
        "variants": [
          {
            "name": "Register",
            "discriminant": 0,
            "template": null,
            "value": {
              "ByIndex": 27
            }
          },
          {
            "name": "Deposit",
            "discriminant": 1,
            "template": null,
            "value": {
              "ByIndex": 29
            }
          },
          {
            "name": "InitiateWithdrawal",
            "discriminant": 2,
            "template": null,
            "value": {
              "ByIndex": 30
            }
          },
          {
            "name": "Withdraw",
            "discriminant": 3,
            "template": null,
            "value": {
              "ByIndex": 31
            }
          }
        ],
        "hide_tag": false
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_CallMessage_Register",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "da_address",
            "silent": false,
            "value": {
              "ByIndex": 28
            },
            "doc": ""
          },
          {
            "display_name": "amount",
            "silent": false,
            "value": {
              "ByIndex": 11
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "Immediate": {
                "ByteArray": {
                  "len": 32,
                  "display": "Hex"
                }
              }
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_CallMessage_Deposit",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "da_address",
            "silent": false,
            "value": {
              "ByIndex": 28
            },
            "doc": ""
          },
          {
            "display_name": "amount",
            "silent": false,
            "value": {
              "ByIndex": 11
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_CallMessage_InitiateWithdrawal",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "da_address",
            "silent": false,
            "value": {
              "ByIndex": 28
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_CallMessage_Withdraw",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "da_address",
            "silent": false,
            "value": {
              "ByIndex": 28
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 33
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Enum": {
        "type_name": "CallMessage",
        "variants": [
          {
            "name": "SetValue",
            "discriminant": 0,
            "template": null,
            "value": {
              "ByIndex": 34
            }
          },
          {
            "name": "SetManyValues",
            "discriminant": 1,
            "template": null,
            "value": {
              "ByIndex": 38
            }
          },
          {
            "name": "AssertVisibleSlotNumber",
            "discriminant": 2,
            "template": null,
            "value": {
              "ByIndex": 39
            }
          }
        ],
        "hide_tag": false
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_CallMessage_SetValue",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "value",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u32",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          },
          {
            "display_name": "gas",
            "silent": false,
            "value": {
              "ByIndex": 35
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Option": {
        "value": {
          "ByIndex": 36
        }
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 37
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Array": {
        "len": 2,
        "value": {
          "Immediate": {
            "Integer": [
              "u64",
              "Decimal"
            ]
          }
        }
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "Immediate": {
                "ByteVec": {
                  "display": "Hex"
                }
              }
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_CallMessage_AssertVisibleSlotNumber",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "expected_visible_slot_number",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 41
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Enum": {
        "type_name": "CallMessage",
        "variants": [
          {
            "name": "RegisterAttester",
            "discriminant": 0,
            "template": null,
            "value": {
              "ByIndex": 42
            }
          },
          {
            "name": "BeginExitAttester",
            "discriminant": 1,
            "template": null,
            "value": null
          },
          {
            "name": "ExitAttester",
            "discriminant": 2,
            "template": null,
            "value": null
          },
          {
            "name": "RegisterChallenger",
            "discriminant": 3,
            "template": null,
            "value": {
              "ByIndex": 43
            }
          },
          {
            "name": "ExitChallenger",
            "discriminant": 4,
            "template": null,
            "value": null
          },
          {
            "name": "DepositAttester",
            "discriminant": 5,
            "template": null,
            "value": {
              "ByIndex": 44
            }
          }
        ],
        "hide_tag": false
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 11
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 11
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 11
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 46
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Enum": {
        "type_name": "CallMessage",
        "variants": [
          {
            "name": "Register",
            "discriminant": 0,
            "template": null,
            "value": {
              "ByIndex": 47
            }
          },
          {
            "name": "Deposit",
            "discriminant": 1,
            "template": null,
            "value": {
              "ByIndex": 48
            }
          },
          {
            "name": "Exit",
            "discriminant": 2,
            "template": null,
            "value": null
          }
        ],
        "hide_tag": false
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 11
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 11
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 50
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Enum": {
        "type_name": "CallMessage",
        "variants": [
          {
            "name": "InsertCredentialId",
            "discriminant": 0,
            "template": null,
            "value": {
              "ByIndex": 51
            }
          }
        ],
        "hide_tag": false
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 52
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 53
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "Immediate": {
                "ByteArray": {
                  "len": 32,
                  "display": "Hex"
                }
              }
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 55
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Enum": {
        "type_name": "NotInstantiable",
        "variants": [],
        "hide_tag": false
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 55
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 55
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 59
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Enum": {
        "type_name": "CallMessage",
        "variants": [
          {
            "name": "RegisterPaymaster",
            "discriminant": 0,
            "template": null,
            "value": {
              "ByIndex": 60
            }
          },
          {
            "name": "SetPayerForSequencer",
            "discriminant": 1,
            "template": null,
            "value": {
              "ByIndex": 73
            }
          },
          {
            "name": "UpdatePolicy",
            "discriminant": 2,
            "template": null,
            "value": {
              "ByIndex": 74
            }
          }
        ],
        "hide_tag": false
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_CallMessage_RegisterPaymaster",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "policy",
            "silent": false,
            "value": {
              "ByIndex": 61
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "PaymasterPolicyInitializer",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "default_payee_policy",
            "silent": false,
            "value": {
              "ByIndex": 62
            },
            "doc": ""
          },
          {
            "display_name": "payees",
            "silent": false,
            "value": {
              "ByIndex": 68
            },
            "doc": ""
          },
          {
            "display_name": "authorized_updaters",
            "silent": false,
            "value": {
              "ByIndex": 17
            },
            "doc": ""
          },
          {
            "display_name": "authorized_sequencers",
            "silent": false,
            "value": {
              "ByIndex": 70
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Enum": {
        "type_name": "PayeePolicy",
        "variants": [
          {
            "name": "Allow",
            "discriminant": 0,
            "template": null,
            "value": {
              "ByIndex": 63
            }
          },
          {
            "name": "Deny",
            "discriminant": 1,
            "template": null,
            "value": null
          }
        ],
        "hide_tag": false
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_PayeePolicy_Allow",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "max_fee",
            "silent": false,
            "value": {
              "ByIndex": 18
            },
            "doc": ""
          },
          {
            "display_name": "gas_limit",
            "silent": false,
            "value": {
              "ByIndex": 35
            },
            "doc": ""
          },
          {
            "display_name": "max_gas_price",
            "silent": false,
            "value": {
              "ByIndex": 64
            },
            "doc": ""
          },
          {
            "display_name": "transaction_limit",
            "silent": false,
            "value": {
              "ByIndex": 67
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Option": {
        "value": {
          "ByIndex": 65
        }
      }
    },
    {
      "Struct": {
        "type_name": "GasPrice",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "value",
            "silent": false,
            "value": {
              "ByIndex": 66
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Array": {
        "len": 2,
        "value": {
          "ByIndex": 11
        }
      }
    },
    {
      "Option": {
        "value": {
          "Immediate": {
            "Integer": [
              "u64",
              "Decimal"
            ]
          }
        }
      }
    },
    {
      "Vec": {
        "value": {
          "ByIndex": 69
        }
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 12
            },
            "silent": false,
            "doc": ""
          },
          {
            "value": {
              "ByIndex": 62
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Enum": {
        "type_name": "AuthorizedSequencers",
        "variants": [
          {
            "name": "All",
            "discriminant": 0,
            "template": null,
            "value": null
          },
          {
            "name": "Some",
            "discriminant": 1,
            "template": null,
            "value": {
              "ByIndex": 71
            }
          }
        ],
        "hide_tag": false
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 72
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Vec": {
        "value": {
          "ByIndex": 28
        }
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_CallMessage_SetPayerForSequencer",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "payer",
            "silent": false,
            "value": {
              "ByIndex": 12
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_CallMessage_UpdatePolicy",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "payer",
            "silent": false,
            "value": {
              "ByIndex": 12
            },
            "doc": ""
          },
          {
            "display_name": "update",
            "silent": false,
            "value": {
              "ByIndex": 75
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "PolicyUpdate",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "sequencer_update",
            "silent": false,
            "value": {
              "ByIndex": 76
            },
            "doc": ""
          },
          {
            "display_name": "updaters_to_add",
            "silent": false,
            "value": {
              "ByIndex": 81
            },
            "doc": ""
          },
          {
            "display_name": "updaters_to_remove",
            "silent": false,
            "value": {
              "ByIndex": 81
            },
            "doc": ""
          },
          {
            "display_name": "payee_policies_to_set",
            "silent": false,
            "value": {
              "ByIndex": 82
            },
            "doc": ""
          },
          {
            "display_name": "payee_policies_to_delete",
            "silent": false,
            "value": {
              "ByIndex": 81
            },
            "doc": ""
          },
          {
            "display_name": "default_policy",
            "silent": false,
            "value": {
              "ByIndex": 83
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Option": {
        "value": {
          "ByIndex": 77
        }
      }
    },
    {
      "Enum": {
        "type_name": "SequencerSetUpdate",
        "variants": [
          {
            "name": "AllowAll",
            "discriminant": 0,
            "template": null,
            "value": null
          },
          {
            "name": "Update",
            "discriminant": 1,
            "template": null,
            "value": {
              "ByIndex": 78
            }
          }
        ],
        "hide_tag": false
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 79
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "AllowedSequencerUpdate",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "to_add",
            "silent": false,
            "value": {
              "ByIndex": 80
            },
            "doc": ""
          },
          {
            "display_name": "to_remove",
            "silent": false,
            "value": {
              "ByIndex": 80
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Option": {
        "value": {
          "ByIndex": 72
        }
      }
    },
    {
      "Option": {
        "value": {
          "ByIndex": 17
        }
      }
    },
    {
      "Option": {
        "value": {
          "ByIndex": 68
        }
      }
    },
    {
      "Option": {
        "value": {
          "ByIndex": 62
        }
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 85
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "CallMessage",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "rlp",
            "silent": false,
            "value": {
              "ByIndex": 86
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "RlpEvmTransaction",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "rlp",
            "silent": false,
            "value": {
              "Immediate": {
                "ByteVec": {
                  "display": "Hex"
                }
              }
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "ByIndex": 88
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Enum": {
        "type_name": "AccessPatternMessages",
        "variants": [
          {
            "name": "WriteCells",
            "discriminant": 0,
            "template": null,
            "value": {
              "ByIndex": 89
            }
          },
          {
            "name": "WriteCustom",
            "discriminant": 1,
            "template": null,
            "value": {
              "ByIndex": 90
            }
          },
          {
            "name": "ReadCells",
            "discriminant": 2,
            "template": null,
            "value": {
              "ByIndex": 92
            }
          },
          {
            "name": "HashBytes",
            "discriminant": 3,
            "template": null,
            "value": {
              "ByIndex": 93
            }
          },
          {
            "name": "HashCustom",
            "discriminant": 4,
            "template": null,
            "value": {
              "ByIndex": 94
            }
          },
          {
            "name": "StoreSignature",
            "discriminant": 5,
            "template": null,
            "value": {
              "ByIndex": 95
            }
          },
          {
            "name": "VerifySignature",
            "discriminant": 6,
            "template": null,
            "value": null
          },
          {
            "name": "VerifyCustomSignature",
            "discriminant": 7,
            "template": null,
            "value": {
              "ByIndex": 96
            }
          },
          {
            "name": "StoreSerializedString",
            "discriminant": 8,
            "template": null,
            "value": {
              "ByIndex": 97
            }
          },
          {
            "name": "DeserializeBytesAsString",
            "discriminant": 9,
            "template": null,
            "value": null
          },
          {
            "name": "DeserializeCustomString",
            "discriminant": 10,
            "template": null,
            "value": {
              "ByIndex": 98
            }
          },
          {
            "name": "DeleteCells",
            "discriminant": 11,
            "template": null,
            "value": {
              "ByIndex": 99
            }
          },
          {
            "name": "SetHook",
            "discriminant": 12,
            "template": null,
            "value": {
              "ByIndex": 100
            }
          },
          {
            "name": "UpdateAdmin",
            "discriminant": 13,
            "template": null,
            "value": {
              "ByIndex": 107
            }
          }
        ],
        "hide_tag": false
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_AccessPatternMessages_WriteCells",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "begin",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          },
          {
            "display_name": "num_cells",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          },
          {
            "display_name": "data_size",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u32",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_AccessPatternMessages_WriteCustom",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "begin",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          },
          {
            "display_name": "content",
            "silent": false,
            "value": {
              "ByIndex": 91
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Vec": {
        "value": {
          "Immediate": "String"
        }
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_AccessPatternMessages_ReadCells",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "begin",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          },
          {
            "display_name": "num_cells",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_AccessPatternMessages_HashBytes",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "filler",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u8",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          },
          {
            "display_name": "size",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u32",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_AccessPatternMessages_HashCustom",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "input",
            "silent": false,
            "value": {
              "Immediate": {
                "ByteVec": {
                  "display": "Hex"
                }
              }
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_AccessPatternMessages_StoreSignature",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "sign",
            "silent": false,
            "value": {
              "ByIndex": 4
            },
            "doc": ""
          },
          {
            "display_name": "pub_key",
            "silent": false,
            "value": {
              "ByIndex": 5
            },
            "doc": ""
          },
          {
            "display_name": "message",
            "silent": false,
            "value": {
              "Immediate": "String"
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_AccessPatternMessages_VerifyCustomSignature",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "sign",
            "silent": false,
            "value": {
              "ByIndex": 4
            },
            "doc": ""
          },
          {
            "display_name": "pub_key",
            "silent": false,
            "value": {
              "ByIndex": 5
            },
            "doc": ""
          },
          {
            "display_name": "message",
            "silent": false,
            "value": {
              "Immediate": "String"
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_AccessPatternMessages_StoreSerializedString",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "input",
            "silent": false,
            "value": {
              "Immediate": {
                "ByteVec": {
                  "display": "Hex"
                }
              }
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_AccessPatternMessages_DeserializeCustomString",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "input",
            "silent": false,
            "value": {
              "Immediate": {
                "ByteVec": {
                  "display": "Hex"
                }
              }
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_AccessPatternMessages_DeleteCells",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "begin",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          },
          {
            "display_name": "num_cells",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_AccessPatternMessages_SetHook",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "pre",
            "silent": false,
            "value": {
              "ByIndex": 101
            },
            "doc": ""
          },
          {
            "display_name": "post",
            "silent": false,
            "value": {
              "ByIndex": 101
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Option": {
        "value": {
          "ByIndex": 102
        }
      }
    },
    {
      "Vec": {
        "value": {
          "ByIndex": 103
        }
      }
    },
    {
      "Enum": {
        "type_name": "HooksConfig",
        "variants": [
          {
            "name": "Read",
            "discriminant": 0,
            "template": null,
            "value": {
              "ByIndex": 104
            }
          },
          {
            "name": "Write",
            "discriminant": 1,
            "template": null,
            "value": {
              "ByIndex": 105
            }
          },
          {
            "name": "Delete",
            "discriminant": 2,
            "template": null,
            "value": {
              "ByIndex": 106
            }
          }
        ],
        "hide_tag": false
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_HooksConfig_Read",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "begin",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          },
          {
            "display_name": "size",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_HooksConfig_Write",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "begin",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          },
          {
            "display_name": "size",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          },
          {
            "display_name": "data_size",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u32",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_HooksConfig_Delete",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "begin",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          },
          {
            "display_name": "size",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "__SovVirtualWallet_AccessPatternMessages_UpdateAdmin",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "new_admin",
            "silent": false,
            "value": {
              "ByIndex": 12
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "TxDetails",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "max_priority_fee_bips",
            "silent": false,
            "value": {
              "ByIndex": 109
            },
            "doc": ""
          },
          {
            "display_name": "max_fee",
            "silent": false,
            "value": {
              "ByIndex": 11
            },
            "doc": ""
          },
          {
            "display_name": "gas_limit",
            "silent": false,
            "value": {
              "ByIndex": 35
            },
            "doc": ""
          },
          {
            "display_name": "chain_id",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          }
        ]
      }
    },
    {
      "Tuple": {
        "template": null,
        "peekable": false,
        "fields": [
          {
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "silent": false,
            "doc": ""
          }
        ]
      }
    },
    {
      "Struct": {
        "type_name": "UnsignedTransaction",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "runtime_call",
            "silent": false,
            "value": {
              "ByIndex": 6
            },
            "doc": ""
          },
          {
            "display_name": "generation",
            "silent": false,
            "value": {
              "Immediate": {
                "Integer": [
                  "u64",
                  "Decimal"
                ]
              }
            },
            "doc": ""
          },
          {
            "display_name": "details",
            "silent": false,
            "value": {
              "ByIndex": 108
            },
            "doc": ""
          }
        ]
      }
    }
  ],
  "root_type_indices": [
    0,
    110,
    6,
    14
  ],
  "chain_data": {
    "chain_id": 4321,
    "chain_name": "TestChain"
  },
  "templates": [
    {},
    {},
    {
      "transfer": {
        "preencoded_bytes": [
          0,
          1
        ],
        "inputs": [
          [
            "to",
            {
              "type_link": {
                "ByIndex": 12
              },
              "offset": 2
            }
          ],
          [
            "amount",
            {
              "type_link": {
                "Immediate": {
                  "Integer": [
                    "u128",
                    {
                      "FixedPoint": {
                        "FromSiblingField": {
                          "field_index": 1,
                          "byte_offset": 31
                        }
                      }
                    }
                  ]
                }
              },
              "offset": 2
            }
          ],
          [
            "token_id",
            {
              "type_link": {
                "ByIndex": 21
              },
              "offset": 2
            }
          ]
        ]
      }
    },
    {}
  ],
  "serde_metadata": [
    {
      "name": "Transaction",
      "fields_or_variants": [
        {
          "name": "versioned_tx"
        }
      ]
    },
    {
      "name": "VersionedTx",
      "fields_or_variants": [
        {
          "name": "V0"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "Version0",
      "fields_or_variants": [
        {
          "name": "signature"
        },
        {
          "name": "pub_key"
        },
        {
          "name": "runtime_call"
        },
        {
          "name": "generation"
        },
        {
          "name": "details"
        }
      ]
    },
    {
      "name": "Ed25519Signature",
      "fields_or_variants": [
        {
          "name": "msg_sig"
        }
      ]
    },
    {
      "name": "Ed25519PublicKey",
      "fields_or_variants": [
        {
          "name": "pub_key"
        }
      ]
    },
    {
      "name": "RuntimeCall",
      "fields_or_variants": [
        {
          "name": "bank"
        },
        {
          "name": "sequencer_registry"
        },
        {
          "name": "value_setter"
        },
        {
          "name": "attester_incentives"
        },
        {
          "name": "prover_incentives"
        },
        {
          "name": "accounts"
        },
        {
          "name": "uniqueness"
        },
        {
          "name": "chain_state"
        },
        {
          "name": "blob_storage"
        },
        {
          "name": "paymaster"
        },
        {
          "name": "evm"
        },
        {
          "name": "access_pattern"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "CallMessage",
      "fields_or_variants": [
        {
          "name": "create_token"
        },
        {
          "name": "transfer"
        },
        {
          "name": "burn"
        },
        {
          "name": "mint"
        },
        {
          "name": "freeze"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_CallMessage_CreateToken",
      "fields_or_variants": [
        {
          "name": "token_name"
        },
        {
          "name": "token_decimals"
        },
        {
          "name": "initial_balance"
        },
        {
          "name": "mint_to_address"
        },
        {
          "name": "admins"
        },
        {
          "name": "supply_cap"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "MultiAddress",
      "fields_or_variants": [
        {
          "name": "Standard"
        },
        {
          "name": "Vm"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "__SovVirtualWallet_CallMessage_Transfer",
      "fields_or_variants": [
        {
          "name": "to"
        },
        {
          "name": "coins"
        }
      ]
    },
    {
      "name": "Coins",
      "fields_or_variants": [
        {
          "name": "amount"
        },
        {
          "name": "token_id"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "__SovVirtualWallet_CallMessage_Burn",
      "fields_or_variants": [
        {
          "name": "coins"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_CallMessage_Mint",
      "fields_or_variants": [
        {
          "name": "coins"
        },
        {
          "name": "mint_to_address"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_CallMessage_Freeze",
      "fields_or_variants": [
        {
          "name": "token_id"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "CallMessage",
      "fields_or_variants": [
        {
          "name": "register"
        },
        {
          "name": "deposit"
        },
        {
          "name": "initiate_withdrawal"
        },
        {
          "name": "withdraw"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_CallMessage_Register",
      "fields_or_variants": [
        {
          "name": "da_address"
        },
        {
          "name": "amount"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "__SovVirtualWallet_CallMessage_Deposit",
      "fields_or_variants": [
        {
          "name": "da_address"
        },
        {
          "name": "amount"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_CallMessage_InitiateWithdrawal",
      "fields_or_variants": [
        {
          "name": "da_address"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_CallMessage_Withdraw",
      "fields_or_variants": [
        {
          "name": "da_address"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "CallMessage",
      "fields_or_variants": [
        {
          "name": "set_value"
        },
        {
          "name": "set_many_values"
        },
        {
          "name": "assert_visible_slot_number"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_CallMessage_SetValue",
      "fields_or_variants": [
        {
          "name": "value"
        },
        {
          "name": "gas"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "__SovVirtualWallet_CallMessage_AssertVisibleSlotNumber",
      "fields_or_variants": [
        {
          "name": "expected_visible_slot_number"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "CallMessage",
      "fields_or_variants": [
        {
          "name": "register_attester"
        },
        {
          "name": "begin_exit_attester"
        },
        {
          "name": "exit_attester"
        },
        {
          "name": "register_challenger"
        },
        {
          "name": "exit_challenger"
        },
        {
          "name": "deposit_attester"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "CallMessage",
      "fields_or_variants": [
        {
          "name": "register"
        },
        {
          "name": "deposit"
        },
        {
          "name": "exit"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "CallMessage",
      "fields_or_variants": [
        {
          "name": "insert_credential_id"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "NotInstantiable",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "CallMessage",
      "fields_or_variants": [
        {
          "name": "register_paymaster"
        },
        {
          "name": "set_payer_for_sequencer"
        },
        {
          "name": "update_policy"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_CallMessage_RegisterPaymaster",
      "fields_or_variants": [
        {
          "name": "policy"
        }
      ]
    },
    {
      "name": "PaymasterPolicyInitializer",
      "fields_or_variants": [
        {
          "name": "default_payee_policy"
        },
        {
          "name": "payees"
        },
        {
          "name": "authorized_updaters"
        },
        {
          "name": "authorized_sequencers"
        }
      ]
    },
    {
      "name": "PayeePolicy",
      "fields_or_variants": [
        {
          "name": "allow"
        },
        {
          "name": "deny"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_PayeePolicy_Allow",
      "fields_or_variants": [
        {
          "name": "max_fee"
        },
        {
          "name": "gas_limit"
        },
        {
          "name": "max_gas_price"
        },
        {
          "name": "transaction_limit"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "GasPrice",
      "fields_or_variants": [
        {
          "name": "value"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "AuthorizedSequencers",
      "fields_or_variants": [
        {
          "name": "all"
        },
        {
          "name": "some"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "__SovVirtualWallet_CallMessage_SetPayerForSequencer",
      "fields_or_variants": [
        {
          "name": "payer"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_CallMessage_UpdatePolicy",
      "fields_or_variants": [
        {
          "name": "payer"
        },
        {
          "name": "update"
        }
      ]
    },
    {
      "name": "PolicyUpdate",
      "fields_or_variants": [
        {
          "name": "sequencer_update"
        },
        {
          "name": "updaters_to_add"
        },
        {
          "name": "updaters_to_remove"
        },
        {
          "name": "payee_policies_to_set"
        },
        {
          "name": "payee_policies_to_delete"
        },
        {
          "name": "default_policy"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "SequencerSetUpdate",
      "fields_or_variants": [
        {
          "name": "allow_all"
        },
        {
          "name": "update"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "AllowedSequencerUpdate",
      "fields_or_variants": [
        {
          "name": "to_add"
        },
        {
          "name": "to_remove"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "CallMessage",
      "fields_or_variants": [
        {
          "name": "rlp"
        }
      ]
    },
    {
      "name": "RlpEvmTransaction",
      "fields_or_variants": [
        {
          "name": "rlp"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "AccessPatternMessages",
      "fields_or_variants": [
        {
          "name": "write_cells"
        },
        {
          "name": "write_custom"
        },
        {
          "name": "read_cells"
        },
        {
          "name": "hash_bytes"
        },
        {
          "name": "hash_custom"
        },
        {
          "name": "store_signature"
        },
        {
          "name": "verify_signature"
        },
        {
          "name": "verify_custom_signature"
        },
        {
          "name": "store_serialized_string"
        },
        {
          "name": "deserialize_bytes_as_string"
        },
        {
          "name": "deserialize_custom_string"
        },
        {
          "name": "delete_cells"
        },
        {
          "name": "set_hook"
        },
        {
          "name": "update_admin"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_AccessPatternMessages_WriteCells",
      "fields_or_variants": [
        {
          "name": "begin"
        },
        {
          "name": "num_cells"
        },
        {
          "name": "data_size"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_AccessPatternMessages_WriteCustom",
      "fields_or_variants": [
        {
          "name": "begin"
        },
        {
          "name": "content"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "__SovVirtualWallet_AccessPatternMessages_ReadCells",
      "fields_or_variants": [
        {
          "name": "begin"
        },
        {
          "name": "num_cells"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_AccessPatternMessages_HashBytes",
      "fields_or_variants": [
        {
          "name": "filler"
        },
        {
          "name": "size"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_AccessPatternMessages_HashCustom",
      "fields_or_variants": [
        {
          "name": "input"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_AccessPatternMessages_StoreSignature",
      "fields_or_variants": [
        {
          "name": "sign"
        },
        {
          "name": "pub_key"
        },
        {
          "name": "message"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_AccessPatternMessages_VerifyCustomSignature",
      "fields_or_variants": [
        {
          "name": "sign"
        },
        {
          "name": "pub_key"
        },
        {
          "name": "message"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_AccessPatternMessages_StoreSerializedString",
      "fields_or_variants": [
        {
          "name": "input"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_AccessPatternMessages_DeserializeCustomString",
      "fields_or_variants": [
        {
          "name": "input"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_AccessPatternMessages_DeleteCells",
      "fields_or_variants": [
        {
          "name": "begin"
        },
        {
          "name": "num_cells"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_AccessPatternMessages_SetHook",
      "fields_or_variants": [
        {
          "name": "pre"
        },
        {
          "name": "post"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "HooksConfig",
      "fields_or_variants": [
        {
          "name": "Read"
        },
        {
          "name": "Write"
        },
        {
          "name": "Delete"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_HooksConfig_Read",
      "fields_or_variants": [
        {
          "name": "begin"
        },
        {
          "name": "size"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_HooksConfig_Write",
      "fields_or_variants": [
        {
          "name": "begin"
        },
        {
          "name": "size"
        },
        {
          "name": "data_size"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_HooksConfig_Delete",
      "fields_or_variants": [
        {
          "name": "begin"
        },
        {
          "name": "size"
        }
      ]
    },
    {
      "name": "__SovVirtualWallet_AccessPatternMessages_UpdateAdmin",
      "fields_or_variants": [
        {
          "name": "new_admin"
        }
      ]
    },
    {
      "name": "TxDetails",
      "fields_or_variants": [
        {
          "name": "max_priority_fee_bips"
        },
        {
          "name": "max_fee"
        },
        {
          "name": "gas_limit"
        },
        {
          "name": "chain_id"
        }
      ]
    },
    {
      "name": "",
      "fields_or_variants": []
    },
    {
      "name": "UnsignedTransaction",
      "fields_or_variants": [
        {
          "name": "runtime_call"
        },
        {
          "name": "generation"
        },
        {
          "name": "details"
        }
      ]
    }
  ]
}"#;
