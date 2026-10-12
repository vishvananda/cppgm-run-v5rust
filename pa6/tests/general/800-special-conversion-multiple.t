// Frozen types-mode special-member declaration responsibility regression.
struct C{operator int(){return 1;}operator long(){return 2;}};
