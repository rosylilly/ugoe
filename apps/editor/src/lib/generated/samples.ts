// このファイルは生成物です。手で編集せず `just gen` で作り直してください (docs/editor.md#型の生成)。
// serde の JSON が生成した型に合うかを型検査で確かめるための値 (ugoe_editor_core::codegen)。
import type { Command, Model } from "./types";

export const models: Model[] = [
  {
    "schema_version": 0,
    "meta": {
      "name": "single_quad",
      "canvas": {
        "width": 256.0,
        "height": 256.0,
        "origin": [
          128.0,
          128.0
        ]
      }
    },
    "parameters": [
      {
        "id": "X",
        "name": "X",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      }
    ],
    "parameter_groups": [],
    "parts": [
      {
        "id": "Root",
        "name": "Root",
        "parent": null,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      }
    ],
    "deformers": [],
    "meshes": [
      {
        "id": "Quad",
        "name": "Quad",
        "part": "Root",
        "parent": null,
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [
            {
              "parameter": "X",
              "keys": [
                -1.0,
                0.0,
                1.0
              ]
            }
          ],
          "keyforms": [
            {
              "positions": [
                [
                  0.0,
                  64.0
                ],
                [
                  128.0,
                  64.0
                ],
                [
                  128.0,
                  192.0
                ],
                [
                  0.0,
                  192.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            },
            {
              "positions": [
                [
                  64.0,
                  64.0
                ],
                [
                  192.0,
                  64.0
                ],
                [
                  192.0,
                  192.0
                ],
                [
                  64.0,
                  192.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            },
            {
              "positions": [
                [
                  128.0,
                  64.0
                ],
                [
                  256.0,
                  64.0
                ],
                [
                  256.0,
                  192.0
                ],
                [
                  128.0,
                  192.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        }
      }
    ],
    "glues": [],
    "textures": [
      {
        "name": "white",
        "width": 1,
        "height": 1
      }
    ],
    "hit_areas": [],
    "pose": {
      "fade_duration": 0.5,
      "groups": []
    },
    "effects": {
      "eye_blink": {
        "parameters": [],
        "interval": 4.0,
        "closing": 0.1,
        "closed": 0.05,
        "opening": 0.15,
        "seed": 0
      },
      "breath": {
        "parameters": []
      },
      "lip_sync": {
        "parameters": []
      }
    }
  },
  {
    "schema_version": 0,
    "meta": {
      "name": "pose_and_effects",
      "canvas": {
        "width": 256.0,
        "height": 256.0,
        "origin": [
          128.0,
          128.0
        ]
      }
    },
    "parameters": [
      {
        "id": "X",
        "name": "X",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "AngleX",
        "name": "AngleX",
        "min": -30.0,
        "max": 30.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "AngleY",
        "name": "AngleY",
        "min": -30.0,
        "max": 30.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "AngleZ",
        "name": "AngleZ",
        "min": -30.0,
        "max": 30.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "BodyAngleX",
        "name": "BodyAngleX",
        "min": -10.0,
        "max": 10.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "BodyAngleY",
        "name": "BodyAngleY",
        "min": -10.0,
        "max": 10.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "BodyAngleZ",
        "name": "BodyAngleZ",
        "min": -10.0,
        "max": 10.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "Breath",
        "name": "Breath",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "EyeLOpen",
        "name": "EyeLOpen",
        "min": 0.0,
        "max": 1.0,
        "default": 1.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "EyeROpen",
        "name": "EyeROpen",
        "min": 0.0,
        "max": 1.0,
        "default": 1.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "EyeLSmile",
        "name": "EyeLSmile",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "EyeRSmile",
        "name": "EyeRSmile",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "EyeBallX",
        "name": "EyeBallX",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "EyeBallY",
        "name": "EyeBallY",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "BrowLY",
        "name": "BrowLY",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "BrowRY",
        "name": "BrowRY",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "MouthForm",
        "name": "MouthForm",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "MouthOpenY",
        "name": "MouthOpenY",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "Cheek",
        "name": "Cheek",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "HairFront",
        "name": "HairFront",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "HairSide",
        "name": "HairSide",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "HairBack",
        "name": "HairBack",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      }
    ],
    "parameter_groups": [],
    "parts": [
      {
        "id": "Root",
        "name": "Root",
        "parent": null,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      },
      {
        "id": "ArmA",
        "name": "ArmA",
        "parent": "Root",
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      },
      {
        "id": "SleeveA",
        "name": "SleeveA",
        "parent": "Root",
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      },
      {
        "id": "ArmB",
        "name": "ArmB",
        "parent": "Root",
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 0.0
            }
          ]
        }
      }
    ],
    "deformers": [],
    "meshes": [
      {
        "id": "Quad",
        "name": "Quad",
        "part": "Root",
        "parent": null,
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [
            {
              "parameter": "X",
              "keys": [
                -1.0,
                0.0,
                1.0
              ]
            }
          ],
          "keyforms": [
            {
              "positions": [
                [
                  0.0,
                  64.0
                ],
                [
                  128.0,
                  64.0
                ],
                [
                  128.0,
                  192.0
                ],
                [
                  0.0,
                  192.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            },
            {
              "positions": [
                [
                  64.0,
                  64.0
                ],
                [
                  192.0,
                  64.0
                ],
                [
                  192.0,
                  192.0
                ],
                [
                  64.0,
                  192.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            },
            {
              "positions": [
                [
                  128.0,
                  64.0
                ],
                [
                  256.0,
                  64.0
                ],
                [
                  256.0,
                  192.0
                ],
                [
                  128.0,
                  192.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        }
      }
    ],
    "glues": [],
    "textures": [
      {
        "name": "white",
        "width": 1,
        "height": 1
      }
    ],
    "hit_areas": [],
    "pose": {
      "fade_duration": 0.5,
      "groups": [
        {
          "entries": [
            {
              "part": "ArmA",
              "links": [
                "SleeveA"
              ]
            },
            {
              "part": "ArmB",
              "links": []
            }
          ]
        }
      ]
    },
    "effects": {
      "eye_blink": {
        "parameters": [
          "EyeLOpen",
          "EyeROpen"
        ],
        "interval": 4.0,
        "closing": 0.1,
        "closed": 0.05,
        "opening": 0.15,
        "seed": 0
      },
      "breath": {
        "parameters": [
          {
            "parameter": "AngleX",
            "offset": 0.0,
            "peak": 15.0,
            "cycle": 6.5345,
            "weight": 0.5
          },
          {
            "parameter": "AngleY",
            "offset": 0.0,
            "peak": 8.0,
            "cycle": 3.5345,
            "weight": 0.5
          },
          {
            "parameter": "AngleZ",
            "offset": 0.0,
            "peak": 10.0,
            "cycle": 5.5345,
            "weight": 0.5
          },
          {
            "parameter": "BodyAngleX",
            "offset": 0.0,
            "peak": 4.0,
            "cycle": 15.5345,
            "weight": 0.5
          },
          {
            "parameter": "Breath",
            "offset": 0.5,
            "peak": 0.5,
            "cycle": 3.2345,
            "weight": 1.0
          }
        ]
      },
      "lip_sync": {
        "parameters": [
          "MouthOpenY"
        ]
      }
    }
  },
  {
    "schema_version": 0,
    "meta": {
      "name": "rotation_under_warp",
      "canvas": {
        "width": 256.0,
        "height": 256.0,
        "origin": [
          128.0,
          128.0
        ]
      }
    },
    "parameters": [
      {
        "id": "Turn",
        "name": "Turn",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      }
    ],
    "parameter_groups": [],
    "parts": [
      {
        "id": "Root",
        "name": "Root",
        "parent": null,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      }
    ],
    "deformers": [
      {
        "id": "Body",
        "name": "Body",
        "part": "Root",
        "parent": null,
        "kind": {
          "type": "warp",
          "rows": 1,
          "cols": 1,
          "keyforms": {
            "bindings": [
              {
                "parameter": "Turn",
                "keys": [
                  0.0,
                  1.0
                ]
              }
            ],
            "keyforms": [
              {
                "points": [
                  [
                    64.0,
                    64.0
                  ],
                  [
                    192.0,
                    64.0
                  ],
                  [
                    64.0,
                    192.0
                  ],
                  [
                    192.0,
                    192.0
                  ]
                ],
                "opacity": 1.0
              },
              {
                "points": [
                  [
                    192.0,
                    64.0
                  ],
                  [
                    192.0,
                    192.0
                  ],
                  [
                    64.0,
                    64.0
                  ],
                  [
                    64.0,
                    192.0
                  ]
                ],
                "opacity": 1.0
              }
            ]
          }
        }
      },
      {
        "id": "Head",
        "name": "Head",
        "part": "Root",
        "parent": "Body",
        "kind": {
          "type": "rotation",
          "keyforms": {
            "bindings": [],
            "keyforms": [
              {
                "origin": [
                  0.5,
                  0.25
                ],
                "angle": 0.0,
                "scale": 1.0,
                "reflect_x": false,
                "reflect_y": false,
                "opacity": 1.0
              }
            ]
          }
        }
      },
      {
        "id": "Cheek",
        "name": "Cheek",
        "part": "Root",
        "parent": "Body",
        "kind": {
          "type": "warp",
          "rows": 1,
          "cols": 1,
          "keyforms": {
            "bindings": [],
            "keyforms": [
              {
                "points": [
                  [
                    0.25,
                    0.5
                  ],
                  [
                    0.75,
                    0.5
                  ],
                  [
                    0.25,
                    1.0
                  ],
                  [
                    0.75,
                    1.0
                  ]
                ],
                "opacity": 1.0
              }
            ]
          }
        }
      }
    ],
    "meshes": [
      {
        "id": "Face",
        "name": "Face",
        "part": "Root",
        "parent": "Head",
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "positions": [
                [
                  0.0,
                  -10.0
                ],
                [
                  10.0,
                  -10.0
                ],
                [
                  10.0,
                  0.0
                ],
                [
                  0.0,
                  0.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        }
      },
      {
        "id": "Blush",
        "name": "Blush",
        "part": "Root",
        "parent": "Cheek",
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "positions": [
                [
                  0.5,
                  0.5
                ],
                [
                  0.5,
                  0.5
                ],
                [
                  0.5,
                  0.5
                ],
                [
                  0.5,
                  0.5
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        }
      }
    ],
    "glues": [],
    "textures": [
      {
        "name": "white",
        "width": 1,
        "height": 1
      }
    ],
    "hit_areas": [],
    "pose": {
      "fade_duration": 0.5,
      "groups": []
    },
    "effects": {
      "eye_blink": {
        "parameters": [],
        "interval": 4.0,
        "closing": 0.1,
        "closed": 0.05,
        "opening": 0.15,
        "seed": 0
      },
      "breath": {
        "parameters": []
      },
      "lip_sync": {
        "parameters": []
      }
    }
  },
  {
    "schema_version": 0,
    "meta": {
      "name": "glued_quads",
      "canvas": {
        "width": 256.0,
        "height": 256.0,
        "origin": [
          128.0,
          128.0
        ]
      }
    },
    "parameters": [
      {
        "id": "Glue",
        "name": "Glue",
        "min": 0.0,
        "max": 1.0,
        "default": 1.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      }
    ],
    "parameter_groups": [],
    "parts": [
      {
        "id": "Root",
        "name": "Root",
        "parent": null,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      }
    ],
    "deformers": [],
    "meshes": [
      {
        "id": "Left",
        "name": "Left",
        "part": "Root",
        "parent": null,
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "positions": [
                [
                  0.0,
                  0.0
                ],
                [
                  100.0,
                  0.0
                ],
                [
                  100.0,
                  100.0
                ],
                [
                  0.0,
                  100.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        }
      },
      {
        "id": "Right",
        "name": "Right",
        "part": "Root",
        "parent": null,
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "positions": [
                [
                  110.0,
                  0.0
                ],
                [
                  210.0,
                  0.0
                ],
                [
                  210.0,
                  100.0
                ],
                [
                  110.0,
                  100.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        }
      }
    ],
    "glues": [
      {
        "id": "Seam",
        "mesh_a": "Left",
        "mesh_b": "Right",
        "pairs": [
          {
            "a": 1,
            "b": 0,
            "weight": 0.5
          },
          {
            "a": 2,
            "b": 3,
            "weight": 0.5
          }
        ],
        "keyforms": {
          "bindings": [
            {
              "parameter": "Glue",
              "keys": [
                0.0,
                1.0
              ]
            }
          ],
          "keyforms": [
            {
              "intensity": 0.0
            },
            {
              "intensity": 1.0
            }
          ]
        }
      },
      {
        "id": "Pull",
        "mesh_a": "Left",
        "mesh_b": "Right",
        "pairs": [
          {
            "a": 1,
            "b": 1,
            "weight": 0.0
          }
        ],
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "intensity": 0.5
            }
          ]
        }
      }
    ],
    "textures": [
      {
        "name": "white",
        "width": 1,
        "height": 1
      }
    ],
    "hit_areas": [],
    "pose": {
      "fade_duration": 0.5,
      "groups": []
    },
    "effects": {
      "eye_blink": {
        "parameters": [],
        "interval": 4.0,
        "closing": 0.1,
        "closed": 0.05,
        "opening": 0.15,
        "seed": 0
      },
      "breath": {
        "parameters": []
      },
      "lip_sync": {
        "parameters": []
      }
    }
  },
  {
    "schema_version": 0,
    "meta": {
      "name": "mixed_masks",
      "canvas": {
        "width": 256.0,
        "height": 256.0,
        "origin": [
          128.0,
          128.0
        ]
      }
    },
    "parameters": [
      {
        "id": "Strength",
        "name": "Strength",
        "min": 0.0,
        "max": 1.0,
        "default": 1.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      }
    ],
    "parameter_groups": [],
    "parts": [
      {
        "id": "Root",
        "name": "Root",
        "parent": null,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      }
    ],
    "deformers": [],
    "meshes": [
      {
        "id": "Face",
        "name": "Face",
        "part": "Root",
        "parent": null,
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "positions": [
                [
                  32.0,
                  32.0
                ],
                [
                  224.0,
                  32.0
                ],
                [
                  224.0,
                  224.0
                ],
                [
                  32.0,
                  224.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 300.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        }
      },
      {
        "id": "Hair",
        "name": "Hair",
        "part": "Root",
        "parent": null,
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "positions": [
                [
                  32.0,
                  32.0
                ],
                [
                  224.0,
                  32.0
                ],
                [
                  224.0,
                  96.0
                ],
                [
                  32.0,
                  96.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 700.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        }
      },
      {
        "id": "Brow",
        "name": "Brow",
        "part": "Root",
        "parent": null,
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [
          {
            "mesh": "Face",
            "invert": false
          },
          {
            "mesh": "Hair",
            "invert": true
          }
        ],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [
            {
              "parameter": "Strength",
              "keys": [
                0.0,
                1.0
              ]
            }
          ],
          "keyforms": [
            {
              "positions": [
                [
                  64.0,
                  64.0
                ],
                [
                  192.0,
                  64.0
                ],
                [
                  192.0,
                  128.0
                ],
                [
                  64.0,
                  128.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": -1.0
            },
            {
              "positions": [
                [
                  64.0,
                  64.0
                ],
                [
                  192.0,
                  64.0
                ],
                [
                  192.0,
                  128.0
                ],
                [
                  64.0,
                  128.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 2.0
            }
          ]
        }
      }
    ],
    "glues": [],
    "textures": [
      {
        "name": "white",
        "width": 1,
        "height": 1
      }
    ],
    "hit_areas": [],
    "pose": {
      "fade_duration": 0.5,
      "groups": []
    },
    "effects": {
      "eye_blink": {
        "parameters": [],
        "interval": 4.0,
        "closing": 0.1,
        "closed": 0.05,
        "opening": 0.15,
        "seed": 0
      },
      "breath": {
        "parameters": []
      },
      "lip_sync": {
        "parameters": []
      }
    }
  },
  {
    "schema_version": 0,
    "meta": {
      "name": "keyform_inference",
      "canvas": {
        "width": 256.0,
        "height": 256.0,
        "origin": [
          128.0,
          128.0
        ]
      }
    },
    "parameters": [
      {
        "id": "AngleX",
        "name": "AngleX",
        "min": -30.0,
        "max": 30.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "AngleY",
        "name": "AngleY",
        "min": -30.0,
        "max": 30.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      }
    ],
    "parameter_groups": [],
    "parts": [
      {
        "id": "Root",
        "name": "Root",
        "parent": null,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      }
    ],
    "deformers": [
      {
        "id": "Face",
        "name": "Face",
        "part": "Root",
        "parent": null,
        "kind": {
          "type": "warp",
          "rows": 1,
          "cols": 1,
          "keyforms": {
            "bindings": [
              {
                "parameter": "AngleX",
                "keys": [
                  -30.0,
                  0.0,
                  30.0
                ]
              },
              {
                "parameter": "AngleY",
                "keys": [
                  -30.0,
                  0.0,
                  30.0
                ]
              }
            ],
            "keyforms": [
              {
                "points": [
                  [
                    64.0,
                    64.0
                  ],
                  [
                    192.0,
                    64.0
                  ],
                  [
                    64.0,
                    192.0
                  ],
                  [
                    192.0,
                    192.0
                  ]
                ],
                "opacity": 1.0
              },
              {
                "points": [
                  [
                    64.0,
                    64.0
                  ],
                  [
                    192.0,
                    64.0
                  ],
                  [
                    64.0,
                    192.0
                  ],
                  [
                    192.0,
                    192.0
                  ]
                ],
                "opacity": 1.0
              },
              {
                "points": [
                  [
                    64.0,
                    64.0
                  ],
                  [
                    192.0,
                    64.0
                  ],
                  [
                    64.0,
                    192.0
                  ],
                  [
                    192.0,
                    192.0
                  ]
                ],
                "opacity": 1.0
              },
              {
                "points": [
                  [
                    64.0,
                    64.0
                  ],
                  [
                    192.0,
                    64.0
                  ],
                  [
                    64.0,
                    192.0
                  ],
                  [
                    192.0,
                    192.0
                  ]
                ],
                "opacity": 1.0
              },
              {
                "points": [
                  [
                    64.0,
                    64.0
                  ],
                  [
                    192.0,
                    64.0
                  ],
                  [
                    64.0,
                    192.0
                  ],
                  [
                    192.0,
                    192.0
                  ]
                ],
                "opacity": 1.0
              },
              {
                "points": [
                  [
                    64.0,
                    64.0
                  ],
                  [
                    192.0,
                    64.0
                  ],
                  [
                    64.0,
                    192.0
                  ],
                  [
                    192.0,
                    192.0
                  ]
                ],
                "opacity": 1.0
              },
              {
                "points": [
                  [
                    64.0,
                    64.0
                  ],
                  [
                    192.0,
                    64.0
                  ],
                  [
                    64.0,
                    192.0
                  ],
                  [
                    192.0,
                    192.0
                  ]
                ],
                "opacity": 1.0
              },
              {
                "points": [
                  [
                    64.0,
                    64.0
                  ],
                  [
                    192.0,
                    64.0
                  ],
                  [
                    64.0,
                    192.0
                  ],
                  [
                    192.0,
                    192.0
                  ]
                ],
                "opacity": 1.0
              },
              {
                "points": [
                  [
                    64.0,
                    64.0
                  ],
                  [
                    192.0,
                    64.0
                  ],
                  [
                    64.0,
                    192.0
                  ],
                  [
                    192.0,
                    192.0
                  ]
                ],
                "opacity": 1.0
              }
            ],
            "origins": [
              "derived",
              "derived",
              "derived",
              "derived",
              "authored",
              "derived",
              "derived",
              "derived",
              "derived"
            ],
            "inference": {
              "method": "depth_rotation",
              "angle_scale": 1.0,
              "pivot": [
                128.0,
                128.0
              ],
              "pivot_depth": 0.0,
              "yaw": "AngleX",
              "pitch": "AngleY"
            }
          },
          "depth": [
            0.0,
            0.0,
            16.0,
            16.0
          ]
        }
      }
    ],
    "meshes": [
      {
        "id": "EyeL",
        "name": "EyeL",
        "part": "Root",
        "parent": "Face",
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "positions": [
                [
                  0.2,
                  0.4
                ],
                [
                  0.4,
                  0.4
                ],
                [
                  0.4,
                  0.5
                ],
                [
                  0.2,
                  0.5
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        },
        "mirror": "EyeR",
        "mirror_axis": 0.5,
        "depth": [
          8.0,
          8.0,
          8.0,
          8.0
        ]
      },
      {
        "id": "EyeR",
        "name": "EyeR",
        "part": "Root",
        "parent": "Face",
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "positions": [
                [
                  0.6,
                  0.4
                ],
                [
                  0.8,
                  0.4
                ],
                [
                  0.8,
                  0.5
                ],
                [
                  0.6,
                  0.5
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        },
        "mirror": "EyeL",
        "mirror_axis": 0.5,
        "depth": [
          8.0,
          8.0,
          8.0,
          8.0
        ]
      },
      {
        "id": "Nose",
        "name": "Nose",
        "part": "Root",
        "parent": "Face",
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "positions": [
                [
                  0.45,
                  0.5
                ],
                [
                  0.55,
                  0.5
                ],
                [
                  0.55,
                  0.7
                ],
                [
                  0.45,
                  0.7
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        }
      }
    ],
    "glues": [],
    "textures": [
      {
        "name": "white",
        "width": 1,
        "height": 1
      }
    ],
    "hit_areas": [],
    "pose": {
      "fade_duration": 0.5,
      "groups": []
    },
    "effects": {
      "eye_blink": {
        "parameters": [],
        "interval": 4.0,
        "closing": 0.1,
        "closed": 0.05,
        "opening": 0.15,
        "seed": 0
      },
      "breath": {
        "parameters": []
      },
      "lip_sync": {
        "parameters": []
      }
    }
  },
  {
    "schema_version": 0,
    "meta": {
      "name": "blend_shapes",
      "canvas": {
        "width": 256.0,
        "height": 256.0,
        "origin": [
          128.0,
          128.0
        ]
      }
    },
    "parameters": [
      {
        "id": "X",
        "name": "X",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "Smile",
        "name": "Smile",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "blend_shape",
        "mirror": null,
        "group": null
      },
      {
        "id": "Wide",
        "name": "Wide",
        "min": 0.0,
        "max": 1.0,
        "default": 0.5,
        "kind": "blend_shape",
        "mirror": null,
        "group": null
      }
    ],
    "parameter_groups": [],
    "parts": [
      {
        "id": "Root",
        "name": "Root",
        "parent": null,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      }
    ],
    "deformers": [
      {
        "id": "Face",
        "name": "Face",
        "part": "Root",
        "parent": null,
        "kind": {
          "type": "warp",
          "rows": 1,
          "cols": 1,
          "keyforms": {
            "bindings": [],
            "keyforms": [
              {
                "points": [
                  [
                    64.0,
                    64.0
                  ],
                  [
                    192.0,
                    64.0
                  ],
                  [
                    64.0,
                    192.0
                  ],
                  [
                    192.0,
                    192.0
                  ]
                ],
                "opacity": 1.0
              }
            ],
            "blend_shapes": [
              {
                "bindings": [
                  {
                    "parameter": "Smile",
                    "keys": [
                      0.0,
                      1.0
                    ]
                  }
                ],
                "keyforms": [
                  {
                    "points": [
                      [
                        64.0,
                        64.0
                      ],
                      [
                        192.0,
                        64.0
                      ],
                      [
                        64.0,
                        192.0
                      ],
                      [
                        192.0,
                        192.0
                      ]
                    ],
                    "opacity": 1.0
                  },
                  {
                    "points": [
                      [
                        64.0,
                        64.0
                      ],
                      [
                        192.0,
                        64.0
                      ],
                      [
                        64.0,
                        224.0
                      ],
                      [
                        192.0,
                        224.0
                      ]
                    ],
                    "opacity": 0.5
                  }
                ]
              }
            ]
          }
        }
      },
      {
        "id": "Jaw",
        "name": "Jaw",
        "part": "Root",
        "parent": null,
        "kind": {
          "type": "rotation",
          "keyforms": {
            "bindings": [],
            "keyforms": [
              {
                "origin": [
                  128.0,
                  128.0
                ],
                "angle": 0.0,
                "scale": 1.0,
                "reflect_x": false,
                "reflect_y": false,
                "opacity": 1.0
              }
            ],
            "blend_shapes": [
              {
                "bindings": [
                  {
                    "parameter": "Wide",
                    "keys": [
                      0.0,
                      1.0
                    ]
                  }
                ],
                "keyforms": [
                  {
                    "origin": [
                      120.0,
                      128.0
                    ],
                    "angle": -10.0,
                    "scale": 0.5,
                    "reflect_x": false,
                    "reflect_y": false,
                    "opacity": 1.0
                  },
                  {
                    "origin": [
                      136.0,
                      128.0
                    ],
                    "angle": 10.0,
                    "scale": 1.5,
                    "reflect_x": false,
                    "reflect_y": false,
                    "opacity": 0.5
                  }
                ]
              }
            ]
          }
        }
      }
    ],
    "meshes": [
      {
        "id": "Mouth",
        "name": "Mouth",
        "part": "Root",
        "parent": null,
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [
            {
              "parameter": "X",
              "keys": [
                -1.0,
                0.0,
                1.0
              ]
            }
          ],
          "keyforms": [
            {
              "positions": [
                [
                  0.0,
                  64.0
                ],
                [
                  128.0,
                  64.0
                ],
                [
                  128.0,
                  192.0
                ],
                [
                  0.0,
                  192.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            },
            {
              "positions": [
                [
                  64.0,
                  64.0
                ],
                [
                  192.0,
                  64.0
                ],
                [
                  192.0,
                  192.0
                ],
                [
                  64.0,
                  192.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            },
            {
              "positions": [
                [
                  128.0,
                  64.0
                ],
                [
                  256.0,
                  64.0
                ],
                [
                  256.0,
                  192.0
                ],
                [
                  128.0,
                  192.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ],
          "blend_shapes": [
            {
              "bindings": [
                {
                  "parameter": "Smile",
                  "keys": [
                    0.0,
                    1.0
                  ]
                }
              ],
              "keyforms": [
                {
                  "positions": [
                    [
                      64.0,
                      64.0
                    ],
                    [
                      192.0,
                      64.0
                    ],
                    [
                      192.0,
                      192.0
                    ],
                    [
                      64.0,
                      192.0
                    ]
                  ],
                  "opacity": 1.0,
                  "draw_order": 500.0,
                  "multiply_color": {
                    "r": 1.0,
                    "g": 1.0,
                    "b": 1.0
                  },
                  "screen_color": {
                    "r": 0.0,
                    "g": 0.0,
                    "b": 0.0
                  },
                  "mask_strength": 1.0
                },
                {
                  "positions": [
                    [
                      56.0,
                      48.0
                    ],
                    [
                      192.0,
                      64.0
                    ],
                    [
                      192.0,
                      192.0
                    ],
                    [
                      64.0,
                      192.0
                    ]
                  ],
                  "opacity": 0.5,
                  "draw_order": 600.0,
                  "multiply_color": {
                    "r": 1.0,
                    "g": 0.5,
                    "b": 0.5
                  },
                  "screen_color": {
                    "r": 0.0,
                    "g": 0.0,
                    "b": 0.25
                  },
                  "mask_strength": 0.5
                }
              ]
            },
            {
              "bindings": [
                {
                  "parameter": "Wide",
                  "keys": [
                    0.0,
                    1.0
                  ]
                }
              ],
              "keyforms": [
                {
                  "positions": [
                    [
                      80.0,
                      64.0
                    ],
                    [
                      176.0,
                      64.0
                    ],
                    [
                      176.0,
                      192.0
                    ],
                    [
                      80.0,
                      192.0
                    ]
                  ],
                  "opacity": 1.0,
                  "draw_order": 500.0,
                  "multiply_color": {
                    "r": 1.0,
                    "g": 1.0,
                    "b": 1.0
                  },
                  "screen_color": {
                    "r": 0.0,
                    "g": 0.0,
                    "b": 0.0
                  },
                  "mask_strength": 1.0
                },
                {
                  "positions": [
                    [
                      48.0,
                      64.0
                    ],
                    [
                      208.0,
                      64.0
                    ],
                    [
                      208.0,
                      192.0
                    ],
                    [
                      48.0,
                      192.0
                    ]
                  ],
                  "opacity": 1.0,
                  "draw_order": 500.0,
                  "multiply_color": {
                    "r": 1.0,
                    "g": 1.0,
                    "b": 1.0
                  },
                  "screen_color": {
                    "r": 0.0,
                    "g": 0.0,
                    "b": 0.0
                  },
                  "mask_strength": 1.0
                }
              ]
            }
          ]
        }
      },
      {
        "id": "Cheek",
        "name": "Cheek",
        "part": "Root",
        "parent": "Face",
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "positions": [
                [
                  0.25,
                  0.25
                ],
                [
                  0.75,
                  0.25
                ],
                [
                  0.75,
                  0.75
                ],
                [
                  0.25,
                  0.75
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        }
      },
      {
        "id": "Chin",
        "name": "Chin",
        "part": "Root",
        "parent": "Jaw",
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "positions": [
                [
                  0.0,
                  0.0
                ],
                [
                  16.0,
                  0.0
                ],
                [
                  16.0,
                  16.0
                ],
                [
                  0.0,
                  16.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        }
      }
    ],
    "glues": [],
    "textures": [
      {
        "name": "white",
        "width": 1,
        "height": 1
      }
    ],
    "hit_areas": [],
    "pose": {
      "fade_duration": 0.5,
      "groups": []
    },
    "effects": {
      "eye_blink": {
        "parameters": [],
        "interval": 4.0,
        "closing": 0.1,
        "closed": 0.05,
        "opening": 0.15,
        "seed": 0
      },
      "breath": {
        "parameters": []
      },
      "lip_sync": {
        "parameters": []
      }
    }
  },
  {
    "schema_version": 0,
    "meta": {
      "name": "smooth_arc",
      "canvas": {
        "width": 256.0,
        "height": 256.0,
        "origin": [
          128.0,
          128.0
        ]
      }
    },
    "parameters": [
      {
        "id": "AngleX",
        "name": "AngleX",
        "min": -30.0,
        "max": 30.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "Squash",
        "name": "Squash",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "blend_shape",
        "mirror": null,
        "group": null
      }
    ],
    "parameter_groups": [],
    "parts": [
      {
        "id": "Root",
        "name": "Root",
        "parent": null,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      }
    ],
    "deformers": [
      {
        "id": "Face",
        "name": "Face",
        "part": "Root",
        "parent": null,
        "kind": {
          "type": "warp",
          "rows": 1,
          "cols": 1,
          "keyforms": {
            "bindings": [
              {
                "parameter": "AngleX",
                "keys": [
                  -30.0,
                  0.0,
                  30.0
                ]
              }
            ],
            "keyforms": [
              {
                "points": [
                  [
                    14.0,
                    77.39746
                  ],
                  [
                    142.0,
                    77.39746
                  ],
                  [
                    14.0,
                    205.39746
                  ],
                  [
                    142.0,
                    205.39746
                  ]
                ],
                "opacity": 1.0
              },
              {
                "points": [
                  [
                    64.0,
                    64.0
                  ],
                  [
                    192.0,
                    64.0
                  ],
                  [
                    64.0,
                    192.0
                  ],
                  [
                    192.0,
                    192.0
                  ]
                ],
                "opacity": 1.0
              },
              {
                "points": [
                  [
                    114.0,
                    77.39746
                  ],
                  [
                    242.0,
                    77.39746
                  ],
                  [
                    114.0,
                    205.39746
                  ],
                  [
                    242.0,
                    205.39746
                  ]
                ],
                "opacity": 1.0
              }
            ],
            "interpolation": "smooth"
          }
        }
      },
      {
        "id": "Jaw",
        "name": "Jaw",
        "part": "Root",
        "parent": null,
        "kind": {
          "type": "rotation",
          "keyforms": {
            "bindings": [
              {
                "parameter": "AngleX",
                "keys": [
                  -30.0,
                  0.0,
                  30.0
                ]
              }
            ],
            "keyforms": [
              {
                "origin": [
                  78.0,
                  41.39746
                ],
                "angle": -30.0,
                "scale": 1.0,
                "reflect_x": false,
                "reflect_y": false,
                "opacity": 1.0
              },
              {
                "origin": [
                  128.0,
                  28.0
                ],
                "angle": 0.0,
                "scale": 1.0,
                "reflect_x": false,
                "reflect_y": false,
                "opacity": 1.0
              },
              {
                "origin": [
                  178.0,
                  41.39746
                ],
                "angle": 30.0,
                "scale": 1.0,
                "reflect_x": false,
                "reflect_y": false,
                "opacity": 1.0
              }
            ],
            "interpolation": "smooth"
          }
        }
      }
    ],
    "meshes": [
      {
        "id": "Cheek",
        "name": "Cheek",
        "part": "Root",
        "parent": null,
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [
            {
              "parameter": "AngleX",
              "keys": [
                -30.0,
                0.0,
                30.0
              ]
            }
          ],
          "keyforms": [
            {
              "positions": [
                [
                  70.0,
                  33.39746
                ],
                [
                  86.0,
                  33.39746
                ],
                [
                  86.0,
                  49.39746
                ],
                [
                  70.0,
                  49.39746
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            },
            {
              "positions": [
                [
                  120.0,
                  20.0
                ],
                [
                  136.0,
                  20.0
                ],
                [
                  136.0,
                  36.0
                ],
                [
                  120.0,
                  36.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            },
            {
              "positions": [
                [
                  170.0,
                  33.39746
                ],
                [
                  186.0,
                  33.39746
                ],
                [
                  186.0,
                  49.39746
                ],
                [
                  170.0,
                  49.39746
                ]
              ],
              "opacity": 0.0,
              "draw_order": 600.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ],
          "interpolation": "smooth",
          "blend_shapes": [
            {
              "bindings": [
                {
                  "parameter": "Squash",
                  "keys": [
                    0.0,
                    0.5,
                    1.0
                  ]
                }
              ],
              "keyforms": [
                {
                  "positions": [
                    [
                      120.0,
                      20.0
                    ],
                    [
                      136.0,
                      20.0
                    ],
                    [
                      136.0,
                      36.0
                    ],
                    [
                      120.0,
                      36.0
                    ]
                  ],
                  "opacity": 1.0,
                  "draw_order": 500.0,
                  "multiply_color": {
                    "r": 1.0,
                    "g": 1.0,
                    "b": 1.0
                  },
                  "screen_color": {
                    "r": 0.0,
                    "g": 0.0,
                    "b": 0.0
                  },
                  "mask_strength": 1.0
                },
                {
                  "positions": [
                    [
                      120.0,
                      30.0
                    ],
                    [
                      136.0,
                      30.0
                    ],
                    [
                      136.0,
                      46.0
                    ],
                    [
                      120.0,
                      46.0
                    ]
                  ],
                  "opacity": 1.0,
                  "draw_order": 500.0,
                  "multiply_color": {
                    "r": 1.0,
                    "g": 1.0,
                    "b": 1.0
                  },
                  "screen_color": {
                    "r": 0.0,
                    "g": 0.0,
                    "b": 0.0
                  },
                  "mask_strength": 1.0
                },
                {
                  "positions": [
                    [
                      120.0,
                      60.0
                    ],
                    [
                      136.0,
                      60.0
                    ],
                    [
                      136.0,
                      76.0
                    ],
                    [
                      120.0,
                      76.0
                    ]
                  ],
                  "opacity": 1.0,
                  "draw_order": 500.0,
                  "multiply_color": {
                    "r": 1.0,
                    "g": 1.0,
                    "b": 1.0
                  },
                  "screen_color": {
                    "r": 0.0,
                    "g": 0.0,
                    "b": 0.0
                  },
                  "mask_strength": 1.0
                }
              ],
              "interpolation": "smooth"
            }
          ]
        }
      }
    ],
    "glues": [],
    "textures": [
      {
        "name": "white",
        "width": 1,
        "height": 1
      }
    ],
    "hit_areas": [],
    "pose": {
      "fade_duration": 0.5,
      "groups": []
    },
    "effects": {
      "eye_blink": {
        "parameters": [],
        "interval": 4.0,
        "closing": 0.1,
        "closed": 0.05,
        "opening": 0.15,
        "seed": 0
      },
      "breath": {
        "parameters": []
      },
      "lip_sync": {
        "parameters": []
      }
    }
  },
  {
    "schema_version": 0,
    "meta": {
      "name": "avatar_profile",
      "canvas": {
        "width": 256.0,
        "height": 256.0,
        "origin": [
          128.0,
          128.0
        ]
      }
    },
    "parameters": [
      {
        "id": "X",
        "name": "X",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "AngleX",
        "name": "AngleX",
        "min": -30.0,
        "max": 30.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "AngleY",
        "name": "AngleY",
        "min": -30.0,
        "max": 30.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "AngleZ",
        "name": "AngleZ",
        "min": -30.0,
        "max": 30.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "BodyAngleX",
        "name": "BodyAngleX",
        "min": -10.0,
        "max": 10.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "BodyAngleY",
        "name": "BodyAngleY",
        "min": -10.0,
        "max": 10.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "BodyAngleZ",
        "name": "BodyAngleZ",
        "min": -10.0,
        "max": 10.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "Breath",
        "name": "Breath",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "EyeLOpen",
        "name": "EyeLOpen",
        "min": 0.0,
        "max": 1.0,
        "default": 1.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "EyeROpen",
        "name": "EyeROpen",
        "min": 0.0,
        "max": 1.0,
        "default": 1.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "EyeLSmile",
        "name": "EyeLSmile",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "EyeRSmile",
        "name": "EyeRSmile",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "EyeBallX",
        "name": "EyeBallX",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "EyeBallY",
        "name": "EyeBallY",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "BrowLY",
        "name": "BrowLY",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "BrowRY",
        "name": "BrowRY",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "MouthForm",
        "name": "MouthForm",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "MouthOpenY",
        "name": "MouthOpenY",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "Cheek",
        "name": "Cheek",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "HairFront",
        "name": "HairFront",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "HairSide",
        "name": "HairSide",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "HairBack",
        "name": "HairBack",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "EyeLWide",
        "name": "EyeLWide",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "EyeRWide",
        "name": "EyeRWide",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "BrowInnerUp",
        "name": "BrowInnerUp",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "BrowLAngle",
        "name": "BrowLAngle",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "BrowRAngle",
        "name": "BrowRAngle",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "MouthX",
        "name": "MouthX",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "JawX",
        "name": "JawX",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "MouthPucker",
        "name": "MouthPucker",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "MouthFunnel",
        "name": "MouthFunnel",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "MouthPress",
        "name": "MouthPress",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "MouthShrug",
        "name": "MouthShrug",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "CheekPuff",
        "name": "CheekPuff",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "TongueOut",
        "name": "TongueOut",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "NoseSneer",
        "name": "NoseSneer",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      }
    ],
    "parameter_groups": [],
    "parts": [
      {
        "id": "Root",
        "name": "Root",
        "parent": null,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      },
      {
        "id": "ArmA",
        "name": "ArmA",
        "parent": "Root",
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      },
      {
        "id": "SleeveA",
        "name": "SleeveA",
        "parent": "Root",
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      },
      {
        "id": "ArmB",
        "name": "ArmB",
        "parent": "Root",
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 0.0
            }
          ]
        }
      }
    ],
    "deformers": [],
    "meshes": [
      {
        "id": "Quad",
        "name": "Quad",
        "part": "Root",
        "parent": null,
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [
            {
              "parameter": "X",
              "keys": [
                -1.0,
                0.0,
                1.0
              ]
            }
          ],
          "keyforms": [
            {
              "positions": [
                [
                  0.0,
                  64.0
                ],
                [
                  128.0,
                  64.0
                ],
                [
                  128.0,
                  192.0
                ],
                [
                  0.0,
                  192.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            },
            {
              "positions": [
                [
                  64.0,
                  64.0
                ],
                [
                  192.0,
                  64.0
                ],
                [
                  192.0,
                  192.0
                ],
                [
                  64.0,
                  192.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            },
            {
              "positions": [
                [
                  128.0,
                  64.0
                ],
                [
                  256.0,
                  64.0
                ],
                [
                  256.0,
                  192.0
                ],
                [
                  128.0,
                  192.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        }
      }
    ],
    "glues": [],
    "textures": [
      {
        "name": "white",
        "width": 1,
        "height": 1
      }
    ],
    "hit_areas": [],
    "pose": {
      "fade_duration": 0.5,
      "groups": [
        {
          "entries": [
            {
              "part": "ArmA",
              "links": [
                "SleeveA"
              ]
            },
            {
              "part": "ArmB",
              "links": []
            }
          ]
        }
      ]
    },
    "effects": {
      "eye_blink": {
        "parameters": [
          "EyeLOpen",
          "EyeROpen"
        ],
        "interval": 4.0,
        "closing": 0.1,
        "closed": 0.05,
        "opening": 0.15,
        "seed": 0
      },
      "breath": {
        "parameters": [
          {
            "parameter": "AngleX",
            "offset": 0.0,
            "peak": 15.0,
            "cycle": 6.5345,
            "weight": 0.5
          },
          {
            "parameter": "AngleY",
            "offset": 0.0,
            "peak": 8.0,
            "cycle": 3.5345,
            "weight": 0.5
          },
          {
            "parameter": "AngleZ",
            "offset": 0.0,
            "peak": 10.0,
            "cycle": 5.5345,
            "weight": 0.5
          },
          {
            "parameter": "BodyAngleX",
            "offset": 0.0,
            "peak": 4.0,
            "cycle": 15.5345,
            "weight": 0.5
          },
          {
            "parameter": "Breath",
            "offset": 0.5,
            "peak": 0.5,
            "cycle": 3.2345,
            "weight": 1.0
          }
        ]
      },
      "lip_sync": {
        "parameters": [
          "MouthOpenY"
        ]
      }
    },
    "avatar": {
      "profile_version": 0,
      "level": 3,
      "meta": {
        "name": "Avatar",
        "version": null,
        "authors": [
          "ugoe"
        ],
        "copyright": null,
        "contact": null,
        "references": [],
        "thumbnail": null,
        "license_url": null,
        "avatar_permission": "only_author",
        "commercial_usage": "personal_non_profit",
        "allow_violent_usage": false,
        "allow_sexual_usage": false,
        "allow_political_or_religious_usage": false,
        "allow_antisocial_or_hate_usage": false,
        "credit_notation": "required",
        "allow_redistribution": false,
        "modification": "prohibited",
        "third_party_licenses": null
      },
      "tracking": [],
      "hit_areas": [],
      "slots": []
    }
  },
  {
    "schema_version": 0,
    "meta": {
      "name": "dress_up_base",
      "canvas": {
        "width": 256.0,
        "height": 256.0,
        "origin": [
          128.0,
          128.0
        ]
      }
    },
    "parameters": [
      {
        "id": "AngleX",
        "name": "AngleX",
        "min": -30.0,
        "max": 30.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "HairFront",
        "name": "HairFront",
        "min": -1.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "BodySway",
        "name": "BodySway",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      },
      {
        "id": "Secret",
        "name": "Secret",
        "min": 0.0,
        "max": 1.0,
        "default": 0.0,
        "kind": "normal",
        "mirror": null,
        "group": null
      }
    ],
    "parameter_groups": [],
    "parts": [
      {
        "id": "Root",
        "name": "Root",
        "parent": null,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      },
      {
        "id": "Hair",
        "name": "Hair",
        "parent": "Root",
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      },
      {
        "id": "Body",
        "name": "Body",
        "parent": "Root",
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "opacity": 1.0
            }
          ]
        }
      }
    ],
    "deformers": [
      {
        "id": "Head",
        "name": "Head",
        "part": "Root",
        "parent": null,
        "kind": {
          "type": "rotation",
          "keyforms": {
            "bindings": [
              {
                "parameter": "AngleX",
                "keys": [
                  -30.0,
                  30.0
                ]
              }
            ],
            "keyforms": [
              {
                "origin": [
                  128.0,
                  96.0
                ],
                "angle": -10.0,
                "scale": 1.0,
                "reflect_x": false,
                "reflect_y": false,
                "opacity": 1.0
              },
              {
                "origin": [
                  128.0,
                  96.0
                ],
                "angle": 10.0,
                "scale": 1.0,
                "reflect_x": false,
                "reflect_y": false,
                "opacity": 1.0
              }
            ]
          }
        }
      },
      {
        "id": "Torso",
        "name": "Torso",
        "part": "Root",
        "parent": null,
        "kind": {
          "type": "warp",
          "rows": 1,
          "cols": 1,
          "keyforms": {
            "bindings": [],
            "keyforms": [
              {
                "points": [
                  [
                    64.0,
                    128.0
                  ],
                  [
                    192.0,
                    128.0
                  ],
                  [
                    64.0,
                    256.0
                  ],
                  [
                    192.0,
                    256.0
                  ]
                ],
                "opacity": 1.0
              }
            ]
          }
        }
      }
    ],
    "meshes": [
      {
        "id": "Face",
        "name": "Face",
        "part": "Root",
        "parent": "Head",
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "positions": [
                [
                  -32.0,
                  -32.0
                ],
                [
                  32.0,
                  -32.0
                ],
                [
                  32.0,
                  32.0
                ],
                [
                  -32.0,
                  32.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 500.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        }
      },
      {
        "id": "HairDefault",
        "name": "HairDefault",
        "part": "Hair",
        "parent": "Head",
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "positions": [
                [
                  -40.0,
                  -48.0
                ],
                [
                  40.0,
                  -48.0
                ],
                [
                  40.0,
                  0.0
                ],
                [
                  -40.0,
                  0.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 600.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        }
      },
      {
        "id": "BodyMesh",
        "name": "BodyMesh",
        "part": "Body",
        "parent": "Torso",
        "texture": 0,
        "uvs": [
          [
            0.0,
            0.0
          ],
          [
            1.0,
            0.0
          ],
          [
            1.0,
            1.0
          ],
          [
            0.0,
            1.0
          ]
        ],
        "indices": [
          0,
          1,
          2,
          0,
          2,
          3
        ],
        "blend": "normal",
        "culling": false,
        "masks": [],
        "mask_mode": "union",
        "mask_only": false,
        "keyforms": {
          "bindings": [],
          "keyforms": [
            {
              "positions": [
                [
                  0.0,
                  0.0
                ],
                [
                  1.0,
                  0.0
                ],
                [
                  1.0,
                  1.0
                ],
                [
                  0.0,
                  1.0
                ]
              ],
              "opacity": 1.0,
              "draw_order": 300.0,
              "multiply_color": {
                "r": 1.0,
                "g": 1.0,
                "b": 1.0
              },
              "screen_color": {
                "r": 0.0,
                "g": 0.0,
                "b": 0.0
              },
              "mask_strength": 1.0
            }
          ]
        }
      }
    ],
    "glues": [],
    "textures": [
      {
        "name": "white",
        "width": 1,
        "height": 1
      }
    ],
    "hit_areas": [],
    "pose": {
      "fade_duration": 0.5,
      "groups": []
    },
    "effects": {
      "eye_blink": {
        "parameters": [],
        "interval": 4.0,
        "closing": 0.1,
        "closed": 0.05,
        "opening": 0.15,
        "seed": 0
      },
      "breath": {
        "parameters": []
      },
      "lip_sync": {
        "parameters": []
      }
    },
    "slots": [
      {
        "id": "Hair",
        "anchor": "Head",
        "draw_order": {
          "min": 550.0,
          "max": 650.0
        },
        "replaces": [
          "Hair"
        ],
        "mask_sources": [
          "Face"
        ]
      },
      {
        "id": "Outfit",
        "anchor": "Torso",
        "draw_order": {
          "min": 300.0,
          "max": 400.0
        },
        "parameters": [
          "BodySway"
        ],
        "part": "Body"
      }
    ]
  }
];

export const commands: Command[] = [
  {
    "op": "add_parameter",
    "parameter": {
      "id": "X",
      "name": "X",
      "min": -1.0,
      "max": 1.0,
      "default": 0.0,
      "kind": "normal",
      "mirror": null,
      "group": null
    }
  },
  {
    "op": "remove_parameter",
    "id": "X"
  },
  {
    "op": "set_parameter_range",
    "id": "X",
    "min": -1.0,
    "default": 0.0,
    "max": 1.0
  },
  {
    "op": "batch",
    "label": "batch",
    "commands": [
      {
        "op": "rename_parameter",
        "id": "X",
        "name": "Angle"
      }
    ]
  },
  {
    "op": "set_parameter_group",
    "id": "X",
    "group": null
  },
  {
    "op": "set_parameter_mirror",
    "id": "X",
    "mirror": "X"
  },
  {
    "op": "add_part",
    "part": {
      "id": "Root",
      "name": "Root",
      "parent": null,
      "keyforms": {
        "bindings": [],
        "keyforms": [
          {
            "opacity": 1.0
          }
        ]
      }
    }
  },
  {
    "op": "remove_part",
    "id": "Root"
  },
  {
    "op": "rename_part",
    "id": "Root",
    "name": "Body"
  },
  {
    "op": "set_part_parent",
    "id": "Root",
    "parent": null
  },
  {
    "op": "set_part_keyforms",
    "id": "Root",
    "keyforms": {
      "bindings": [],
      "keyforms": [
        {
          "opacity": 1.0
        }
      ]
    }
  },
  {
    "op": "batch",
    "label": "deformers",
    "commands": [
      {
        "op": "add_deformer",
        "deformer": {
          "id": "Arm",
          "name": "Arm",
          "part": "Root",
          "parent": null,
          "kind": {
            "type": "rotation",
            "keyforms": {
              "bindings": [
                {
                  "parameter": "Angle",
                  "keys": [
                    -30.0,
                    30.0
                  ]
                }
              ],
              "keyforms": [
                {
                  "origin": [
                    128.0,
                    128.0
                  ],
                  "angle": -30.0,
                  "scale": 1.0,
                  "reflect_x": false,
                  "reflect_y": false,
                  "opacity": 1.0
                },
                {
                  "origin": [
                    128.0,
                    128.0
                  ],
                  "angle": 30.0,
                  "scale": 1.0,
                  "reflect_x": false,
                  "reflect_y": false,
                  "opacity": 1.0
                }
              ]
            }
          }
        }
      },
      {
        "op": "add_deformer",
        "deformer": {
          "id": "Hand",
          "name": "Hand",
          "part": "Root",
          "parent": "Arm",
          "kind": {
            "type": "rotation",
            "keyforms": {
              "bindings": [
                {
                  "parameter": "Flip",
                  "keys": [
                    0.0,
                    1.0
                  ]
                }
              ],
              "keyforms": [
                {
                  "origin": [
                    64.0,
                    0.0
                  ],
                  "angle": 0.0,
                  "scale": 0.5,
                  "reflect_x": false,
                  "reflect_y": false,
                  "opacity": 1.0
                },
                {
                  "origin": [
                    64.0,
                    0.0
                  ],
                  "angle": 0.0,
                  "scale": 0.5,
                  "reflect_x": true,
                  "reflect_y": false,
                  "opacity": 1.0
                }
              ]
            }
          }
        }
      },
      {
        "op": "add_deformer",
        "deformer": {
          "id": "Cloth",
          "name": "Cloth",
          "part": "Root",
          "parent": "Arm",
          "kind": {
            "type": "warp",
            "rows": 1,
            "cols": 1,
            "keyforms": {
              "bindings": [],
              "keyforms": [
                {
                  "points": [
                    [
                      0.0,
                      20.0
                    ],
                    [
                      40.0,
                      20.0
                    ],
                    [
                      0.0,
                      40.0
                    ],
                    [
                      40.0,
                      40.0
                    ]
                  ],
                  "opacity": 1.0
                }
              ]
            }
          }
        }
      }
    ]
  },
  {
    "op": "add_deformer",
    "deformer": {
      "id": "Arm",
      "name": "Arm",
      "part": "Root",
      "parent": null,
      "kind": {
        "type": "rotation",
        "keyforms": {
          "bindings": [
            {
              "parameter": "Angle",
              "keys": [
                -30.0,
                30.0
              ]
            }
          ],
          "keyforms": [
            {
              "origin": [
                128.0,
                128.0
              ],
              "angle": -30.0,
              "scale": 1.0,
              "reflect_x": false,
              "reflect_y": false,
              "opacity": 1.0
            },
            {
              "origin": [
                128.0,
                128.0
              ],
              "angle": 30.0,
              "scale": 1.0,
              "reflect_x": false,
              "reflect_y": false,
              "opacity": 1.0
            }
          ]
        }
      }
    }
  },
  {
    "op": "remove_deformer",
    "id": "Arm"
  },
  {
    "op": "rename_deformer",
    "id": "Arm",
    "name": "Warp"
  },
  {
    "op": "set_deformer_parent",
    "id": "Arm",
    "parent": "Arm"
  },
  {
    "op": "set_deformer_part",
    "id": "Arm",
    "part": "Root"
  },
  {
    "op": "set_warp_keyforms",
    "id": "Arm",
    "keyforms": {
      "bindings": [],
      "keyforms": [
        {
          "points": [
            [
              0.0,
              20.0
            ],
            [
              40.0,
              20.0
            ],
            [
              0.0,
              40.0
            ],
            [
              40.0,
              40.0
            ]
          ],
          "opacity": 1.0
        }
      ]
    }
  },
  {
    "op": "set_rotation_keyforms",
    "id": "Arm",
    "keyforms": {
      "bindings": [
        {
          "parameter": "Angle",
          "keys": [
            -30.0,
            30.0
          ]
        }
      ],
      "keyforms": [
        {
          "origin": [
            128.0,
            128.0
          ],
          "angle": -30.0,
          "scale": 1.0,
          "reflect_x": false,
          "reflect_y": false,
          "opacity": 1.0
        },
        {
          "origin": [
            128.0,
            128.0
          ],
          "angle": 30.0,
          "scale": 1.0,
          "reflect_x": false,
          "reflect_y": false,
          "opacity": 1.0
        }
      ]
    }
  },
  {
    "op": "add_mesh",
    "mesh": {
      "id": "EyeL",
      "name": "EyeL",
      "part": "Root",
      "parent": "Face",
      "texture": 0,
      "uvs": [
        [
          0.0,
          0.0
        ],
        [
          1.0,
          0.0
        ],
        [
          1.0,
          1.0
        ],
        [
          0.0,
          1.0
        ]
      ],
      "indices": [
        0,
        1,
        2,
        0,
        2,
        3
      ],
      "blend": "normal",
      "culling": false,
      "masks": [],
      "mask_mode": "union",
      "mask_only": false,
      "keyforms": {
        "bindings": [],
        "keyforms": [
          {
            "positions": [
              [
                0.2,
                0.4
              ],
              [
                0.4,
                0.4
              ],
              [
                0.4,
                0.5
              ],
              [
                0.2,
                0.5
              ]
            ],
            "opacity": 1.0,
            "draw_order": 500.0,
            "multiply_color": {
              "r": 1.0,
              "g": 1.0,
              "b": 1.0
            },
            "screen_color": {
              "r": 0.0,
              "g": 0.0,
              "b": 0.0
            },
            "mask_strength": 1.0
          }
        ]
      },
      "mirror": "EyeR",
      "mirror_axis": 0.5,
      "depth": [
        8.0,
        8.0,
        8.0,
        8.0
      ]
    }
  },
  {
    "op": "remove_mesh",
    "id": "EyeL"
  },
  {
    "op": "rename_mesh",
    "id": "EyeL",
    "name": "Face"
  },
  {
    "op": "set_mesh_properties",
    "id": "EyeL"
  },
  {
    "op": "set_mesh_properties",
    "id": "EyeL",
    "part": "Root",
    "texture_name": "face.png",
    "blend": "multiply",
    "culling": true,
    "masks": [
      {
        "mesh": "EyeL",
        "invert": true
      }
    ],
    "mask_mode": "intersection",
    "mask_only": false,
    "lit": false
  },
  {
    "op": "set_mesh_parent",
    "id": "EyeL",
    "parent": "Arm"
  },
  {
    "op": "set_mesh_keyform",
    "id": "EyeL",
    "cell": {},
    "keyform": {
      "positions": [
        [
          0.2,
          0.4
        ],
        [
          0.4,
          0.4
        ],
        [
          0.4,
          0.5
        ],
        [
          0.2,
          0.5
        ]
      ],
      "opacity": 1.0,
      "draw_order": 500.0,
      "multiply_color": {
        "r": 1.0,
        "g": 1.0,
        "b": 1.0
      },
      "screen_color": {
        "r": 0.0,
        "g": 0.0,
        "b": 0.0
      },
      "mask_strength": 1.0
    }
  },
  {
    "op": "set_mesh_keyforms",
    "id": "EyeL",
    "keyforms": {
      "bindings": [],
      "keyforms": [
        {
          "positions": [
            [
              0.2,
              0.4
            ],
            [
              0.4,
              0.4
            ],
            [
              0.4,
              0.5
            ],
            [
              0.2,
              0.5
            ]
          ],
          "opacity": 1.0,
          "draw_order": 500.0,
          "multiply_color": {
            "r": 1.0,
            "g": 1.0,
            "b": 1.0
          },
          "screen_color": {
            "r": 0.0,
            "g": 0.0,
            "b": 0.0
          },
          "mask_strength": 1.0
        }
      ]
    }
  },
  {
    "op": "reorder_mesh",
    "id": "EyeL",
    "before": null
  },
  {
    "op": "add_texture",
    "name": "face.png",
    "png": [
      137,
      80,
      78,
      71
    ]
  },
  {
    "op": "set_texture_image",
    "name": "face.png",
    "png": [
      137,
      80,
      78,
      71
    ]
  },
  {
    "op": "remove_texture",
    "name": "face.png"
  },
  {
    "op": "set_canvas",
    "width": 1024.0,
    "height": 2048.0,
    "origin": [
      512.0,
      1024.0
    ]
  },
  {
    "op": "regenerate_mesh",
    "id": "EyeL",
    "options": {
      "alpha_threshold": 8,
      "margin": 4.0,
      "edge_spacing": 24.0,
      "interior_spacing": 48.0,
      "min_island_area": 0
    }
  },
  {
    "op": "add_keyform_key",
    "target": {
      "kind": "mesh",
      "id": "EyeL"
    },
    "parameter": "AngleX",
    "key": 15.0
  },
  {
    "op": "remove_keyform_key",
    "target": {
      "kind": "deformer",
      "id": "Arm"
    },
    "parameter": "Angle",
    "key": 30.0
  },
  {
    "op": "set_mesh_draw_order",
    "id": "EyeL",
    "draw_order": 750.0
  },
  {
    "op": "set_keyform_cell",
    "target": {
      "kind": "mesh",
      "id": "EyeL"
    },
    "cell": {},
    "keyform": {
      "kind": "mesh",
      "positions": [
        [
          0.2,
          0.4
        ],
        [
          0.4,
          0.4
        ],
        [
          0.4,
          0.5
        ],
        [
          0.2,
          0.5
        ]
      ],
      "opacity": 1.0,
      "draw_order": 500.0,
      "multiply_color": {
        "r": 1.0,
        "g": 1.0,
        "b": 1.0
      },
      "screen_color": {
        "r": 0.0,
        "g": 0.0,
        "b": 0.0
      },
      "mask_strength": 1.0
    }
  },
  {
    "op": "set_keyform_cell",
    "target": {
      "kind": "deformer",
      "id": "Arm"
    },
    "cell": {
      "Angle": -30.0
    },
    "keyform": {
      "kind": "rotation",
      "origin": [
        128.0,
        128.0
      ],
      "angle": -30.0,
      "scale": 1.0,
      "reflect_x": false,
      "reflect_y": false,
      "opacity": 1.0
    }
  },
  {
    "op": "set_keyform_cell",
    "target": {
      "kind": "deformer",
      "id": "Hand"
    },
    "cell": {
      "Flip": 0.0
    },
    "keyform": {
      "kind": "rotation",
      "origin": [
        64.0,
        0.0
      ],
      "angle": 0.0,
      "scale": 0.5,
      "reflect_x": false,
      "reflect_y": false,
      "opacity": 1.0
    }
  },
  {
    "op": "set_keyform_cell",
    "target": {
      "kind": "deformer",
      "id": "Cloth"
    },
    "cell": {},
    "keyform": {
      "kind": "warp",
      "points": [
        [
          0.0,
          20.0
        ],
        [
          40.0,
          20.0
        ],
        [
          0.0,
          40.0
        ],
        [
          40.0,
          40.0
        ]
      ],
      "opacity": 1.0
    }
  },
  {
    "op": "set_keyform_cell",
    "target": {
      "kind": "part",
      "id": "Root"
    },
    "cell": {},
    "keyform": {
      "kind": "part",
      "opacity": 1.0
    }
  },
  {
    "op": "set_keyform_cell",
    "target": {
      "kind": "glue",
      "id": "Glue"
    },
    "cell": {},
    "keyform": {
      "kind": "glue",
      "intensity": 0.5
    }
  },
  {
    "op": "infer_keyforms",
    "target": {
      "kind": "mesh",
      "id": "EyeL"
    },
    "method": "depth_rotation",
    "overwrite_derived": true
  },
  {
    "op": "mirror_keyforms",
    "source": {
      "kind": "deformer",
      "id": "Arm"
    },
    "target": {
      "kind": "glue",
      "id": "Glue"
    },
    "overwrite_authored": false
  },
  {
    "op": "mirror_keyforms",
    "source": {
      "kind": "part",
      "id": "Root"
    },
    "target": {
      "kind": "part",
      "id": "Root"
    },
    "axis": 512.0,
    "tolerance": 0.5,
    "overwrite_authored": true
  },
  {
    "op": "mark_keyform_authored",
    "target": {
      "kind": "mesh",
      "id": "EyeL"
    },
    "cells": [
      {}
    ]
  },
  {
    "op": "mark_keyform_derived",
    "target": {
      "kind": "mesh",
      "id": "EyeL"
    },
    "cells": [
      {},
      {}
    ]
  },
  {
    "op": "move_mesh_vertices",
    "id": "EyeL",
    "moves": [
      {
        "index": 2,
        "delta": [
          4.0,
          -8.0
        ]
      }
    ]
  },
  {
    "op": "add_mesh_vertex",
    "id": "EyeL",
    "position": [
      512.0,
      300.0
    ]
  },
  {
    "op": "remove_mesh_vertices",
    "id": "EyeL",
    "indices": [
      0,
      3
    ]
  },
  {
    "op": "create_warp_deformer",
    "name": "Body warp",
    "children": [
      {
        "kind": "mesh",
        "id": "EyeL"
      },
      {
        "kind": "deformer",
        "id": "Arm"
      }
    ],
    "rows": 3,
    "cols": 3
  },
  {
    "op": "create_warp_deformer",
    "name": "",
    "parts": [
      "Head"
    ],
    "rows": 2,
    "cols": 2
  },
  {
    "op": "create_rotation_deformer",
    "id": "Neck",
    "name": "Neck",
    "children": [
      {
        "kind": "mesh",
        "id": "EyeL"
      }
    ],
    "origin": [
      512.0,
      400.0
    ],
    "parent": "Body"
  },
  {
    "op": "reparent_preserving_look",
    "element": {
      "kind": "mesh",
      "id": "EyeL"
    },
    "new_parent": null
  },
  {
    "op": "move_rotation_origin",
    "id": "Neck",
    "delta": [
      0.0,
      12.5
    ]
  },
  {
    "op": "unwrap_deformer",
    "id": "Neck"
  },
  {
    "op": "set_keyform_inference",
    "target": {
      "kind": "mesh",
      "id": "EyeL"
    },
    "inference": {
      "method": "depth_rotation",
      "angle_scale": 0.8,
      "pivot": [
        512.0,
        400.0
      ],
      "pivot_depth": -40.0,
      "yaw": "AngleX"
    }
  },
  {
    "op": "set_keyform_inference",
    "target": {
      "kind": "deformer",
      "id": "Arm"
    }
  },
  {
    "op": "set_mirror_pair",
    "target": {
      "kind": "mesh",
      "id": "EyeL"
    },
    "mirror": "EyeR",
    "axis": 512.0
  },
  {
    "op": "set_mirror_pair",
    "target": {
      "kind": "deformer",
      "id": "Arm"
    }
  },
  {
    "op": "set_depth",
    "target": {
      "kind": "mesh",
      "id": "EyeL"
    },
    "depth": [
      0.0,
      4.0,
      12.0,
      4.0
    ]
  },
  {
    "op": "set_depth",
    "target": {
      "kind": "deformer",
      "id": "Arm"
    }
  },
  {
    "op": "apply_depth_preset",
    "target": {
      "kind": "mesh",
      "id": "EyeL"
    },
    "preset": {
      "kind": "uniform",
      "value": 2.0
    }
  },
  {
    "op": "apply_depth_preset",
    "target": {
      "kind": "deformer",
      "id": "Arm"
    },
    "preset": {
      "kind": "dome",
      "peak": 20.0
    }
  },
  {
    "op": "apply_depth_preset",
    "target": {
      "kind": "mesh",
      "id": "EyeL"
    },
    "preset": {
      "kind": "points",
      "indices": [
        0,
        2
      ],
      "value": 8.0
    }
  },
  {
    "op": "retriangulate_mesh",
    "id": "EyeL"
  },
  {
    "op": "set_keyform_interpolation",
    "target": {
      "kind": "mesh",
      "id": "EyeL"
    },
    "interpolation": "smooth"
  },
  {
    "op": "set_keyform_interpolation",
    "target": {
      "kind": "deformer",
      "id": "Arm"
    },
    "blend_shape": "Smile",
    "interpolation": "linear"
  },
  {
    "op": "add_motion",
    "motion": {
      "name": "Idle",
      "duration": 2.0,
      "looping": true,
      "fade_in": 0.5,
      "fade_out": 0.5,
      "curves": [
        {
          "target": {
            "kind": "parameter",
            "id": "X"
          },
          "segments": [
            {
              "type": "start",
              "p": {
                "t": 0.0,
                "v": 0.0
              }
            },
            {
              "type": "bezier",
              "c1": {
                "t": 0.3,
                "v": 0.0
              },
              "c2": {
                "t": 0.7,
                "v": 1.0
              },
              "to": {
                "t": 1.0,
                "v": 1.0
              }
            },
            {
              "type": "linear",
              "to": {
                "t": 2.0,
                "v": 0.0
              }
            }
          ],
          "fade_in": null,
          "fade_out": 0.2
        }
      ],
      "events": [
        {
          "t": 1.0,
          "value": "step"
        }
      ]
    }
  },
  {
    "op": "remove_motion",
    "name": "Idle"
  },
  {
    "op": "rename_motion",
    "name": "Idle",
    "new_name": "Idle_2"
  },
  {
    "op": "set_motion_properties",
    "name": "Idle",
    "duration": 3.0,
    "looping": false,
    "fade_out": 1.0
  },
  {
    "op": "add_motion_curve",
    "motion": "Idle",
    "curve": {
      "target": {
        "kind": "part_opacity",
        "id": "Root"
      },
      "segments": [
        {
          "type": "start",
          "p": {
            "t": 0.0,
            "v": 1.0
          }
        },
        {
          "type": "stepped",
          "to": {
            "t": 1.0,
            "v": 0.0
          }
        },
        {
          "type": "inverse_stepped",
          "to": {
            "t": 1.5,
            "v": 1.0
          }
        }
      ],
      "fade_in": 0.1,
      "fade_out": null
    }
  },
  {
    "op": "remove_motion_curve",
    "motion": "Idle",
    "target": {
      "kind": "effect_group",
      "id": "EyeBlink"
    }
  },
  {
    "op": "set_motion_curve_fade",
    "motion": "Idle",
    "target": {
      "kind": "parameter",
      "id": "X"
    },
    "fade_in": 0.25
  },
  {
    "op": "set_motion_key",
    "motion": "Idle",
    "target": {
      "kind": "parameter",
      "id": "X"
    },
    "point": {
      "t": 0.5,
      "v": 0.25
    }
  },
  {
    "op": "move_motion_key",
    "motion": "Idle",
    "target": {
      "kind": "parameter",
      "id": "X"
    },
    "t": 1.0,
    "to": {
      "t": 1.2,
      "v": 0.8
    }
  },
  {
    "op": "remove_motion_key",
    "motion": "Idle",
    "target": {
      "kind": "parameter",
      "id": "X"
    },
    "t": 2.0
  },
  {
    "op": "set_motion_segment",
    "motion": "Idle",
    "target": {
      "kind": "parameter",
      "id": "X"
    },
    "t": 1.0,
    "segment": {
      "type": "bezier",
      "c1": {
        "t": 0.2,
        "v": 0.0
      }
    }
  },
  {
    "op": "set_motion_segment",
    "motion": "Idle",
    "target": {
      "kind": "parameter",
      "id": "X"
    },
    "t": 2.0,
    "segment": {
      "type": "inverse_stepped"
    }
  },
  {
    "op": "set_motion_events",
    "motion": "Idle",
    "events": [
      {
        "t": 0.5,
        "value": "blink"
      }
    ]
  },
  {
    "op": "add_expression",
    "expression": {
      "name": "Smile",
      "fade_in": 0.3,
      "fade_out": 0.3,
      "parameters": [
        {
          "parameter": "X",
          "value": 0.5,
          "op": "add"
        }
      ]
    }
  },
  {
    "op": "remove_expression",
    "name": "Smile"
  },
  {
    "op": "rename_expression",
    "name": "Smile",
    "new_name": "Happy"
  },
  {
    "op": "set_expression_properties",
    "name": "Smile",
    "fade_in": 0.1
  },
  {
    "op": "set_expression_parameters",
    "name": "Smile",
    "parameters": [
      {
        "parameter": "X",
        "value": 1.0,
        "op": "overwrite"
      }
    ]
  },
  {
    "op": "set_physics_world",
    "gravity": [
      0.0,
      1.0
    ],
    "wind": [
      0.2,
      0.0
    ],
    "fps": 60.0
  },
  {
    "op": "add_physics_rig",
    "rig": {
      "id": "Hair",
      "name": "髪",
      "inputs": [],
      "outputs": [
        {
          "target": {
            "type": "deformer_angle",
            "deformer": "Hair1"
          },
          "particle": 1,
          "source": "angle",
          "scale": 1.0,
          "weight": 100.0,
          "reflect": false
        },
        {
          "target": {
            "type": "deformer_angle",
            "deformer": "Hair2"
          },
          "particle": 2,
          "source": "angle",
          "scale": 1.0,
          "weight": 100.0,
          "reflect": false
        },
        {
          "target": {
            "type": "deformer_angle",
            "deformer": "Hair3"
          },
          "particle": 3,
          "source": "angle",
          "scale": 1.0,
          "weight": 100.0,
          "reflect": false
        }
      ],
      "particles": [
        {
          "position": [
            0.0,
            0.0
          ],
          "mobility": 0.9,
          "delay": 0.9,
          "acceleration": 1.5,
          "radius": 0.0,
          "collision_radius": 0.0
        },
        {
          "position": [
            0.0,
            30.0
          ],
          "mobility": 0.9,
          "delay": 0.9,
          "acceleration": 1.5,
          "radius": 30.0,
          "collision_radius": 2.0
        },
        {
          "position": [
            0.0,
            60.0
          ],
          "mobility": 0.9,
          "delay": 0.9,
          "acceleration": 1.5,
          "radius": 30.0,
          "collision_radius": 2.0
        },
        {
          "position": [
            0.0,
            90.0
          ],
          "mobility": 0.9,
          "delay": 0.9,
          "acceleration": 1.5,
          "radius": 30.0,
          "collision_radius": 2.0
        }
      ],
      "normalization": {
        "position": {
          "min": -10.0,
          "default": 0.0,
          "max": 10.0
        },
        "angle": {
          "min": -10.0,
          "default": 0.0,
          "max": 10.0
        }
      },
      "colliders": [
        "Shoulder"
      ]
    }
  },
  {
    "op": "create_physics_rig",
    "name": "後ろ髪",
    "deformers": [
      "Hair1",
      "Hair2"
    ]
  },
  {
    "op": "remove_physics_rig",
    "id": "Hair"
  },
  {
    "op": "rename_physics_rig",
    "id": "Hair",
    "name": "前髪"
  },
  {
    "op": "set_physics_inputs",
    "rig": "Skirt",
    "inputs": [
      {
        "parameter": "AngleX",
        "source": "x",
        "weight": 100.0,
        "reflect": false
      }
    ]
  },
  {
    "op": "set_physics_outputs",
    "rig": "Skirt",
    "outputs": [
      {
        "target": {
          "type": "vertices",
          "owner": {
            "warp": "Cloth"
          },
          "weights": [
            0.0,
            0.0,
            1.0,
            1.0
          ]
        },
        "particle": 1,
        "source": "x",
        "scale": 1.0,
        "weight": 100.0,
        "reflect": false
      },
      {
        "target": {
          "type": "vertices",
          "owner": {
            "warp": "Cloth"
          },
          "weights": [
            0.0,
            0.0,
            1.0,
            1.0
          ]
        },
        "particle": 1,
        "source": "y",
        "scale": 1.0,
        "weight": 100.0,
        "reflect": false
      },
      {
        "target": {
          "type": "vertices",
          "owner": {
            "mesh": "Ribbon"
          },
          "weights": [
            0.0,
            0.0,
            1.0,
            1.0
          ]
        },
        "particle": 1,
        "source": "x",
        "scale": 1.0,
        "weight": 100.0,
        "reflect": false
      }
    ]
  },
  {
    "op": "set_physics_outputs",
    "rig": "BodySway",
    "outputs": [
      {
        "target": {
          "type": "parameter",
          "id": "BodyAngleX"
        },
        "particle": 1,
        "source": "x",
        "scale": 0.5,
        "weight": 100.0,
        "reflect": false,
        "blend": "add"
      }
    ]
  },
  {
    "op": "set_physics_particles",
    "rig": "Hair",
    "particles": [
      {
        "position": [
          0.0,
          0.0
        ],
        "mobility": 0.9,
        "delay": 0.9,
        "acceleration": 1.5,
        "radius": 0.0,
        "collision_radius": 0.0
      },
      {
        "position": [
          0.0,
          30.0
        ],
        "mobility": 0.9,
        "delay": 0.9,
        "acceleration": 1.5,
        "radius": 30.0,
        "collision_radius": 2.0
      },
      {
        "position": [
          0.0,
          60.0
        ],
        "mobility": 0.9,
        "delay": 0.9,
        "acceleration": 1.5,
        "radius": 30.0,
        "collision_radius": 2.0
      },
      {
        "position": [
          0.0,
          90.0
        ],
        "mobility": 0.9,
        "delay": 0.9,
        "acceleration": 1.5,
        "radius": 30.0,
        "collision_radius": 2.0
      }
    ]
  },
  {
    "op": "set_physics_normalization",
    "rig": "Hair",
    "normalization": {
      "position": {
        "min": -10.0,
        "default": 0.0,
        "max": 10.0
      },
      "angle": {
        "min": -10.0,
        "default": 0.0,
        "max": 10.0
      }
    }
  },
  {
    "op": "set_physics_rig_colliders",
    "rig": "Hair",
    "colliders": [
      "Shoulder"
    ]
  },
  {
    "op": "set_physics_frame",
    "rig": "Skirt",
    "frame": "Body"
  },
  {
    "op": "set_physics_frame",
    "rig": "Hair"
  },
  {
    "op": "add_physics_collider",
    "collider": {
      "id": "Arm",
      "name": "肩",
      "shape": {
        "type": "capsule",
        "a": [
          35.0,
          -50.0
        ],
        "b": [
          70.0,
          -30.0
        ],
        "radius": 8.0
      }
    }
  },
  {
    "op": "remove_physics_collider",
    "id": "Arm"
  },
  {
    "op": "set_physics_collider",
    "collider": {
      "id": "Shoulder",
      "name": "肩",
      "attach": "Body",
      "shape": {
        "type": "capsule",
        "a": [
          35.0,
          -50.0
        ],
        "b": [
          70.0,
          -30.0
        ],
        "radius": 8.0
      }
    }
  },
  {
    "op": "reorder_physics_rig",
    "id": "Skirt",
    "before": "Hair"
  },
  {
    "op": "set_parameter_internal",
    "id": "AngleX",
    "internal": true
  },
  {
    "op": "apply_physics_template",
    "template": "body_follow",
    "options": {
      "create_missing": false,
      "blend": "add"
    }
  },
  {
    "op": "apply_physics_template",
    "template": "neck_follow",
    "options": {
      "create_missing": true,
      "blend": "overwrite",
      "neck": "Body"
    }
  },
  {
    "op": "create_draw_clone",
    "source": "Quad",
    "name": ""
  },
  {
    "op": "sync_draw_clone",
    "id": "Quad_Clone"
  },
  {
    "op": "create_glue",
    "mesh_a": "Front",
    "mesh_b": "Back",
    "distance": 4.0
  },
  {
    "op": "set_glue",
    "glue": {
      "id": "Glue_Front_Back",
      "mesh_a": "Front",
      "mesh_b": "Back",
      "pairs": [
        {
          "a": 0,
          "b": 1,
          "weight": 0.5
        }
      ],
      "keyforms": {
        "bindings": [],
        "keyforms": [
          {
            "intensity": 1.0
          }
        ]
      }
    },
    "distance": 4.0
  },
  {
    "op": "remove_glue",
    "id": "Glue_Front_Back"
  },
  {
    "op": "set_avatar_profile",
    "profile": {
      "profile_version": 0,
      "level": 3,
      "meta": {
        "name": "Avatar",
        "version": null,
        "authors": [
          "ugoe"
        ],
        "copyright": null,
        "contact": null,
        "references": [],
        "thumbnail": null,
        "license_url": null,
        "avatar_permission": "only_author",
        "commercial_usage": "personal_non_profit",
        "allow_violent_usage": false,
        "allow_sexual_usage": false,
        "allow_political_or_religious_usage": false,
        "allow_antisocial_or_hate_usage": false,
        "credit_notation": "required",
        "allow_redistribution": false,
        "modification": "prohibited",
        "third_party_licenses": null
      },
      "tracking": [],
      "hit_areas": [],
      "slots": []
    }
  },
  {
    "op": "set_avatar_profile",
    "profile": null
  },
  {
    "op": "update_avatar_profile",
    "level": 2,
    "meta": {
      "name": "Avatar",
      "version": null,
      "authors": [
        "ugoe"
      ],
      "copyright": null,
      "contact": null,
      "references": [],
      "thumbnail": null,
      "license_url": null,
      "avatar_permission": "only_author",
      "commercial_usage": "personal_non_profit",
      "allow_violent_usage": false,
      "allow_sexual_usage": false,
      "allow_political_or_religious_usage": false,
      "allow_antisocial_or_hate_usage": false,
      "credit_notation": "required",
      "allow_redistribution": false,
      "modification": "prohibited",
      "third_party_licenses": null
    },
    "tracking": [
      {
        "source": "standard",
        "input": "EyeBallX",
        "scale": -1.0,
        "offset": 0.0
      }
    ],
    "look_at": {
      "x": "EyeBallX",
      "y": "EyeBallY",
      "range_x": 30.0,
      "range_y": 30.0
    },
    "hit_areas": [
      {
        "name": "Head",
        "hit_area": "Head"
      }
    ],
    "slots": []
  },
  {
    "op": "update_avatar_profile",
    "look_at": null
  },
  {
    "op": "set_hit_areas",
    "hit_areas": [
      {
        "id": "Head",
        "mesh": "Quad"
      }
    ]
  },
  {
    "op": "set_normal_map",
    "texture": "face.png",
    "normal_map": "face_normal",
    "png": [
      137,
      80,
      78,
      71
    ]
  },
  {
    "op": "set_normal_map",
    "texture": "face.png",
    "normal_map": null
  },
  {
    "op": "set_lighting",
    "enabled": true
  },
  {
    "op": "set_effects",
    "effects": {
      "eye_blink": {
        "parameters": [
          "EyeLOpen",
          "EyeROpen"
        ],
        "interval": 4.0,
        "closing": 0.1,
        "closed": 0.05,
        "opening": 0.15,
        "seed": 0
      },
      "breath": {
        "parameters": [
          {
            "parameter": "AngleX",
            "offset": 0.0,
            "peak": 15.0,
            "cycle": 6.5345,
            "weight": 0.5
          },
          {
            "parameter": "AngleY",
            "offset": 0.0,
            "peak": 8.0,
            "cycle": 3.5345,
            "weight": 0.5
          },
          {
            "parameter": "AngleZ",
            "offset": 0.0,
            "peak": 10.0,
            "cycle": 5.5345,
            "weight": 0.5
          },
          {
            "parameter": "BodyAngleX",
            "offset": 0.0,
            "peak": 4.0,
            "cycle": 15.5345,
            "weight": 0.5
          },
          {
            "parameter": "Breath",
            "offset": 0.5,
            "peak": 0.5,
            "cycle": 3.2345,
            "weight": 1.0
          }
        ]
      },
      "lip_sync": {
        "parameters": [
          "MouthOpenY"
        ]
      }
    }
  },
  {
    "op": "set_pose",
    "pose": {
      "fade_duration": 0.5,
      "groups": [
        {
          "entries": [
            {
              "part": "ArmA",
              "links": [
                "SleeveA"
              ]
            },
            {
              "part": "ArmB",
              "links": []
            }
          ]
        }
      ]
    }
  },
  {
    "op": "add_slot",
    "slot": {
      "id": "Accessory",
      "anchor": "Head",
      "draw_order": {
        "min": 700.0,
        "max": 750.0
      },
      "replaces": [
        "Hair"
      ],
      "mask_sources": [
        "Face"
      ]
    }
  },
  {
    "op": "set_slot",
    "id": "Outfit",
    "slot": {
      "id": "Outfit",
      "anchor": "Torso",
      "draw_order": {
        "min": 300.0,
        "max": 400.0
      },
      "parameters": [
        "BodySway"
      ],
      "part": "Body"
    }
  },
  {
    "op": "remove_slot",
    "id": "Hair"
  },
  {
    "op": "set_module_settings",
    "name": "LongHair",
    "slot": "Hair",
    "requires": {
      "avatar_level": 1,
      "slots": [
        "Outfit"
      ],
      "parameters": [
        "BodySway"
      ]
    },
    "keep_look": true
  },
  {
    "op": "set_layer_correction",
    "path": [
      "Face",
      "Eye L"
    ],
    "correction": {
      "label": "eye_white",
      "side": "left"
    }
  },
  {
    "op": "set_layer_correction",
    "path": [
      "Face",
      "Eye L"
    ],
    "correction": null
  }
];
