pub const CHAIN_HASH: [u8; 32] = [75, 78, 59, 51, 217, 50, 109, 163, 110, 211, 240, 7, 18, 31, 151, 91, 50, 213, 0, 164, 232, 77, 92, 82, 176, 196, 98, 20, 47, 183, 153, 60];

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
              "ByIndex": 51
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
              "ByIndex": 26
            }
          },
          {
            "name": "AttesterIncentives",
            "discriminant": 2,
            "template": null,
            "value": {
              "ByIndex": 33
            }
          },
          {
            "name": "ProverIncentives",
            "discriminant": 3,
            "template": null,
            "value": {
              "ByIndex": 38
            }
          },
          {
            "name": "Accounts",
            "discriminant": 4,
            "template": null,
            "value": {
              "ByIndex": 42
            }
          },
          {
            "name": "ChainState",
            "discriminant": 5,
            "template": null,
            "value": {
              "ByIndex": 47
            }
          },
          {
            "name": "BlobStorage",
            "discriminant": 6,
            "template": null,
            "value": {
              "ByIndex": 49
            }
          },
          {
            "name": "Svm",
            "discriminant": 7,
            "template": null,
            "value": {
              "ByIndex": 50
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
              "ByIndex": 20
            }
          },
          {
            "name": "Burn",
            "discriminant": 2,
            "template": null,
            "value": {
              "ByIndex": 23
            }
          },
          {
            "name": "Mint",
            "discriminant": 3,
            "template": null,
            "value": {
              "ByIndex": 24
            }
          },
          {
            "name": "Freeze",
            "discriminant": 4,
            "template": null,
            "value": {
              "ByIndex": 25
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
              "ByIndex": 18
            },
            "doc": ""
          },
          {
            "display_name": "supply_cap",
            "silent": false,
            "value": {
              "ByIndex": 19
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
              "ByIndex": 17
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
                "ByteVec": {
                  "display": {
                    "Bech32": {
                      "prefix": "solana"
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
              "ByIndex": 21
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
              "ByIndex": 22
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
              "ByIndex": 21
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
              "ByIndex": 21
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
              "ByIndex": 22
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
              "ByIndex": 27
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
              "ByIndex": 28
            }
          },
          {
            "name": "Deposit",
            "discriminant": 1,
            "template": null,
            "value": {
              "ByIndex": 30
            }
          },
          {
            "name": "InitiateWithdrawal",
            "discriminant": 2,
            "template": null,
            "value": {
              "ByIndex": 31
            }
          },
          {
            "name": "Withdraw",
            "discriminant": 3,
            "template": null,
            "value": {
              "ByIndex": 32
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
              "ByIndex": 29
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
              "ByIndex": 29
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
              "ByIndex": 29
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
              "ByIndex": 29
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
              "ByIndex": 34
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
              "ByIndex": 35
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
              "ByIndex": 36
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
              "ByIndex": 37
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
              "ByIndex": 39
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
              "ByIndex": 40
            }
          },
          {
            "name": "Deposit",
            "discriminant": 1,
            "template": null,
            "value": {
              "ByIndex": 41
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
              "ByIndex": 43
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
              "ByIndex": 45
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
              "ByIndex": 48
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
              "ByIndex": 48
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
        "type_name": "TxDetails",
        "template": null,
        "peekable": false,
        "fields": [
          {
            "display_name": "max_priority_fee_bips",
            "silent": false,
            "value": {
              "ByIndex": 52
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
              "ByIndex": 53
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
      "Option": {
        "value": {
          "ByIndex": 54
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
              "ByIndex": 55
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
              "ByIndex": 51
            },
            "doc": ""
          }
        ]
      }
    }
  ],
  "root_type_indices": [
    0,
    56,
    6,
    12
  ],
  "chain_data": {
    "chain_id": 4321,
    "chain_name": "nitrosvm-testnet"
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
                "ByIndex": 22
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
          "name": "attester_incentives"
        },
        {
          "name": "prover_incentives"
        },
        {
          "name": "accounts"
        },
        {
          "name": "chain_state"
        },
        {
          "name": "blob_storage"
        },
        {
          "name": "svm"
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
