template<int n> struct value {};
const int T = 0;
value<(T < 2)> relational;
value<(T < 1) + (2 > 1)> sibling_regions;
