// Frozen types-mode special-member declaration responsibility regression.
struct C{int C();};void f(){C();new C;}
