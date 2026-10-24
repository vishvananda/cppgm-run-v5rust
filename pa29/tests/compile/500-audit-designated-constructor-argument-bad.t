// PA29 audit: designated list legality after target selection.
struct C { C(int); int x; };
struct Other { void* y; };
int pick(C); int pick(Other);
int use() { return pick({.x=1}); }
