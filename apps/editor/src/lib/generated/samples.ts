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
    "texture": 0,
    "blend": "multiply",
    "culling": true,
    "masks": [
      {
        "mesh": "EyeL",
        "invert": true
      }
    ],
    "mask_mode": "intersection",
    "mask_only": false
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
    "op": "infer_keyforms",
    "target": {
      "kind": "mesh",
      "id": "EyeL"
    },
    "method": "depth_rotation"
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
  }
];
