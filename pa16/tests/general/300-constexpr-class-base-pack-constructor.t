struct left
{
  constexpr left(int value) : left_value(value) {}
  int left_value;
};

struct right
{
  constexpr right(int value) : right_value(value) {}
  int right_value;
};

template<class... Base>
struct pair : Base...
{
  constexpr pair(Base... value) : Base(value)... {}
};

constexpr pair<left, right> make_value()
{
  return pair<left, right>{left(3), right(5)};
}

static_assert(make_value().left_value == 3, "");
static_assert(make_value().right_value == 5, "");

int main() {}
