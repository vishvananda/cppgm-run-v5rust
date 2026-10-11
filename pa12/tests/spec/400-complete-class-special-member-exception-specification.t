// Constructor names do not hide the injected class name in sizeof(type).
struct holder
{
  holder() noexcept(sizeof(holder) > 0);
  ~holder() noexcept(sizeof(holder) > 0);
  operator int() noexcept(sizeof(holder) > 0);
  struct nested
  {
    ~nested() noexcept(sizeof(holder) > 0);
  };
  nested member;
};

int main() { return 0; }
