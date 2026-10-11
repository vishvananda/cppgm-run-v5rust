// VALIDATION: compile-fail
// Deleted constructors participate in ranking and diagnose when selected.

struct view
{
  view(char const *) {}
  view(decltype(nullptr)) = delete;
};

int main()
{
  view value(nullptr);
  return 0;
}
