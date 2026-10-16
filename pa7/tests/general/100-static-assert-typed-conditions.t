// Semantic assertion conditions use ordinary value typing and contextual bool.
int object;
int function();
static_assert(&object, "object address");
static_assert(function, "function address");
static_assert(!nullptr, "null pointer");
static_assert((true ? &object : nullptr), "pointer conditional");
static_assert((true || (1 / 0)), "skipped constant operation");
int main() { return 0; }
