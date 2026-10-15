// expect exit 0
// An uncalled internal function leaves a constructor declaration, not an object root.
namespace {
struct source { operator short(); };
struct target { target(source&); };
void unused(source& value) { target object = value; }
}
int main() { return 0; }
