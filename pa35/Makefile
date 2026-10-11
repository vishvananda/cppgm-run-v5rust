# PA35: optimizing the compiler. See README.md.
#
#   make test                   every gate group in order, stopping at the first failing group
#   make test-correctness | test-speed | test-codegen   one gate group
#   make test-progress          rungs passed toward every gate, from the last gate runs (scripts/progress.pl)
#   make test-smoke [SMOKE_SEEDS=5]   the correctness group alone, into obj/smoke
#   make quick [LEVELS=O0,O3]   fast check of the current tree (scripts/quick.pl)
#   make freeze LABEL=L         obj/bin/{seed,self}-L from the current tree
#   make measure LABEL=L [SIM=1]  callgrind both at -O0 and -O3 into obj/results/L
#   make abba LABEL=L [LEVEL=-O3] [BLOCKS=12]  paired timing of seed-L and self-L
#   make identity OLD=A NEW=B   generated output identical, compiler work A -> B
#   make kernels | behavior | generated | speed | scaling   one gate family, dev/cppgm++
#   make profile [LEVEL=-O0] [INPUT=file.cpp] [SECONDS=30]   where dev/cppgm++ spends its time
#   make parity [REFERENCE=compiler] [SIM=1] [NO_CACHE=1]   this compiler beside the reference on this machine (scripts/parity.pl)

PERL ?= perl
S := scripts
LABEL ?= work
LEVEL ?= -O3
BLOCKS ?= 12
SEEDS ?= 20
LEVELS ?= O0,O3
SECONDS ?= 30
SMOKE_SEEDS ?= 5

.PHONY: test check test-correctness test-speed test-codegen test-progress test-smoke quick freeze measure abba identity kernels behavior generated speed scaling profile parity clean

test check:
	$(PERL) $(S)/check.pl --seeds $(SEEDS)

test-correctness test-speed test-codegen:
	$(PERL) $(S)/check.pl --group $(patsubst test-%,%,$@) --seeds $(SEEDS)

test-progress:
	$(PERL) $(S)/progress.pl

# The correctness group (workload compiles, behavior, generated) with fewer
# generated seeds, written apart from obj/check so test-progress is unaffected.
test-smoke:
	$(PERL) $(S)/check.pl --group correctness --seeds $(SMOKE_SEEDS) --out obj/smoke

quick:
	$(PERL) $(S)/quick.pl --levels $(LEVELS)

freeze:
	$(PERL) $(S)/freeze.pl --label $(LABEL)

measure:
	$(PERL) $(S)/measure.pl --seed obj/bin/seed-$(LABEL) --self obj/bin/self-$(LABEL) \
		--out obj/results/$(LABEL)$(if $(SIM),-sim) $(if $(SIM),--sim)

abba:
	$(PERL) $(S)/abba.pl obj/bin/seed-$(LABEL) obj/bin/self-$(LABEL) --level=$(LEVEL) --blocks $(BLOCKS)

identity:
	@test -n "$(OLD)" -a -n "$(NEW)" || { echo "usage: make identity OLD=compiler NEW=compiler"; exit 2; }
	$(PERL) $(S)/identity.pl $(OLD) $(NEW)

kernels:
	$(PERL) $(S)/kernels.pl

behavior:
	$(PERL) $(S)/behavior.pl

generated:
	$(PERL) $(S)/generated.pl --seeds $(SEEDS)

speed:
	$(PERL) $(S)/speed.pl

scaling:
	$(PERL) $(S)/scaling.pl --keep obj/scaling-sources

# LEVEL's default above is abba's; profile defaults to -O0, as profile.pl does.
PROFILE_LEVEL = $(if $(filter file,$(origin LEVEL)),-O0,$(LEVEL))

profile:
	$(PERL) $(S)/profile.pl --level $(if $(filter -O%,$(PROFILE_LEVEL)),$(PROFILE_LEVEL),-O0) --seconds $(SECONDS) $(if $(INPUT),--input $(INPUT))

parity:
	$(PERL) $(S)/parity.pl $(if $(REFERENCE),--reference $(REFERENCE)) $(if $(SIM),--sim) $(if $(NO_CACHE),--no-cache)

clean:
	rm -rf obj

-include maintainer/Makefile
