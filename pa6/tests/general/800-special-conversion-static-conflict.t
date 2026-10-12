// Frozen types-mode special-member declaration responsibility regression.
struct C{static operator int();operator int()const;};
