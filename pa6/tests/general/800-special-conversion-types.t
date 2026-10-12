// Frozen types-mode special-member declaration responsibility regression.
struct C{operator const int&()const; operator int*()&;operator int&&()&&;};
