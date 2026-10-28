struct One { char bytes[281474976710657]; };
struct Sixteen { char bytes[281474976710672]; };
static_assert(sizeof(One) == 281474976710657UL, "");
void storage() { One first; Sixteen second; }
One* step(One* value) { return value + 1; }
int main() { return 0; }
