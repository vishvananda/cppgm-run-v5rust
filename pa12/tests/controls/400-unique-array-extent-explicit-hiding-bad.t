// N3485 5.3.4p6, 12.3p5: an explicit derived declaration hides the
// base conversion but cannot supply an implicit array extent; expect reject
struct Base { operator int(); };
struct Extent : Base { explicit operator int(); };
int* allocate(Extent& extent) { return new int[extent]; }
