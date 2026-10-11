# The release check, for course maintainers; omitted from the student export.
#
#   make ref-test [CALIBRATION_SEEDS=100]   every gate on the course solution
#       in the course build and the student build, with the release margins
#       enforced in both; writes calibration.tsv

CALIBRATION_SEEDS ?= 100

.PHONY: ref-test

ref-test:
	$(PERL) maintainer/calibrate.pl --seeds $(CALIBRATION_SEEDS)
