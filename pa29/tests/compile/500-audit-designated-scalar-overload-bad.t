// PA29 audit: designated list legality after target selection.
struct A { int x; };
int pick(int); int pick(A);
int use() { return pick({.x=1}); }
