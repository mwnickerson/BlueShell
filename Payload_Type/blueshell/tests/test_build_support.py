import importlib.util
import ast
import json
import sys
import unittest
from pathlib import Path

MODULE_PATH = (
    Path(__file__).parents[1]
    / "blueshell"
    / "mythic"
    / "agent_functions"
    / "build_support.py"
)
SPEC = importlib.util.spec_from_file_location("build_support", MODULE_PATH)
build_support = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
sys.modules[SPEC.name] = build_support
SPEC.loader.exec_module(build_support)
normalize_crypto = build_support.normalize_crypto
output_filename = build_support.output_filename
serialize_c2 = build_support.serialize_c2


class FakeC2:
    def __init__(self, name, p2p, parameters):
        self.name = name
        self.p2p = p2p
        self.parameters = parameters

    def get_c2profile(self):
        return {"name": self.name, "is_p2p": self.p2p}

    def get_parameters_dict(self):
        return self.parameters


class BuildSupportTests(unittest.TestCase):
    def test_external_agent_config(self):
        root = Path(__file__).parents[3]
        config = json.loads((root / "config.json").read_text())
        self.assertFalse(config["exclude_payload_type"])
        self.assertTrue(config["exclude_c2_profiles"])

    def test_only_concrete_payload_types_inherit_payload_type(self):
        builder = MODULE_PATH.with_name("builder.py")
        tree = ast.parse(builder.read_text())
        payload_classes = []
        for node in tree.body:
            if isinstance(node, ast.ClassDef):
                bases = {
                    base.id
                    for base in node.bases
                    if isinstance(base, ast.Name)
                }
                if "PayloadType" in bases:
                    payload_classes.append(node.name)
        self.assertEqual(
            payload_classes,
            ["BlueShellStage0", "BlueShellStage1"],
        )
        for node in tree.body:
            if isinstance(node, ast.ClassDef) and node.name in payload_classes:
                self.assertEqual(len(node.bases), 1)

    def test_extensions(self):
        self.assertEqual(output_filename("x.bin", "dll"), "x.dll")
        self.assertEqual(output_filename("x.bin", "service_exe"), "x.exe")

    def test_crypto_normalization(self):
        self.assertEqual(
            normalize_crypto({"enc_key": "a", "dec_key": "b"}),
            {"enc_key": "a", "dec_key": "b"},
        )
        self.assertEqual(normalize_crypto(None), {"enc_key": None, "dec_key": None})

    def test_c2_serialization(self):
        result = serialize_c2(
            [FakeC2("smb", True, {"AESPSK": {"enc_key": "a", "dec_key": "b"}})]
        )
        self.assertEqual(result[0]["name"], "smb")
        self.assertTrue(result[0]["is_p2p"])
        self.assertEqual(result[0]["parameters"]["AESPSK"]["enc_key"], "a")


if __name__ == "__main__":
    unittest.main()
