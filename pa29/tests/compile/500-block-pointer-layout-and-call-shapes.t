// Block identities retain their own ABI spelling and use the ordinary call facts.
typedef int (^Block)(int);
static_assert(sizeof(Block) == 8, "size");
static_assert(alignof(Block) == 8, "alignment");
static_assert(!__is_pointer(Block), "distinct type");
static_assert(!__is_scalar(Block), "oracle trait");
struct Pair { Block value; int tag; };
static_assert(sizeof(Pair) == 16, "member layout");
int invoke_plain(int (*function)(void *, int), void *literal, int value) { return function(literal, value); }
int invoke(Block block, int value) { return (block)(value); }
int qualified(int (^const _Nullable block)(int), int value) { return block(value); }
int variadic(int (^block)(int, ...), int value) { return block(value, 2.5); }
struct Big { long a, b, c; };
Big aggregate(Big (^block)(int), int value) { return block(value); }
void discarded(void (^block)()) { block(); }
int &reference(int &(^block)()) { return block(); }
const char *pretty(Block value) { return __PRETTY_FUNCTION__; }
