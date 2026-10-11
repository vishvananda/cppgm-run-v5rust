struct constants
{
  static constexpr double scale = 2.5;
};

static_assert(constants::scale > 2.0, "floating member constant");

int main()
{
  return 0;
}
