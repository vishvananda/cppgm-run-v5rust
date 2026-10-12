// Frozen types-mode special-member declaration responsibility regression.
struct C{int C();C* next;void f(){C();}};
