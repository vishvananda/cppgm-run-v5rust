// Frozen types-mode special-member declaration responsibility regression.
struct C{operator int()&;operator int()&&;};
