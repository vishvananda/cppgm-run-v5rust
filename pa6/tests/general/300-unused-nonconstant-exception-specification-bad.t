// An ordinary member's specification is checked even when it is unused.
int probe();
struct holder
{
  void unused() noexcept(probe() > 0);
};

int main() { return 0; }
