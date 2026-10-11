// AUDIT-ID: EH-SPEC-SET
// AUDIT-EXPECT: compile
// N3485 15.4/3: order and duplicates do not change the allowed type set.
void compatible() throw(int, double);
void compatible() throw(double, int);
void compatible() throw(int, double, int);
