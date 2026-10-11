// One function in two subobjects is two final overriders ([class.virtual]/2):
// Outer reaches the shared virtual base Root through the virtual Inner (via
// Left) and through its own non-virtual Inner, and each overrides id.
struct Root { virtual int id() { return 1; } };
struct Inner : virtual Root { int id() { return 2; } };
struct Left : virtual Inner {};
struct Outer : Left, Inner {};

int main()
{
  Outer outer;
  return 0;
}
