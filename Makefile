.PHONY: validate python-check stage0-check stage1-check

validate: python-check stage0-check stage1-check

python-check:
	python3 -m compileall -q Payload_Type/blueshell
	python3 -m unittest discover -s Payload_Type/blueshell/tests -v
	python3 scripts/validate_layout.py

stage0-check:
	cmake -S Payload_Type/blueshell/blueshell/agent_code/stage0 \
		-B /tmp/blueshell-stage0-build \
		-DCMAKE_BUILD_TYPE=Release
	cmake --build /tmp/blueshell-stage0-build --parallel

stage1-check:
	cargo test --manifest-path \
		Payload_Type/blueshell/blueshell/agent_code/stage1/Cargo.toml
