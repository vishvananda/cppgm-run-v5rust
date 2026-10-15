// expect exit 0
// The frozen implementation permits a conversion using-declaration outside
// a class, through the ordinary declaration-introduction path.
struct Source { operator int(); };
using Source::operator int;
int main() {}
