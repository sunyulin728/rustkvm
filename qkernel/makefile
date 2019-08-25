arch ?= x86_64
kernel := build/kernel-$(arch).bin
target ?= $(arch)-qkernel
qkernel := target/$(target)/debug/libqkernel.a

linker_script := src/arch/$(arch)/linker.ld
assembly_source_files := $(wildcard src/arch/$(arch)/*.s)
assembly_object_files := $(patsubst src/arch/$(arch)/%.s, \
	build/arch/$(arch)/%.o, $(assembly_source_files))

all: $(kernel)

$(kernel): kernel $(assembly_object_files) 
	@ld -n --gc-sections -T $(linker_script) -o $(kernel) \
		$(assembly_object_files) $(qkernel)

kernel: 
	@cargo xbuild --target x86_64-qkernel.json

build/arch/$(arch)/%.o: src/arch/$(arch)/%.s
	@mkdir -p $(shell dirname $@)
	$(AS) $^ -o $@

