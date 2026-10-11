struct Control
{
  int references;
  int alive;
  int value;
};

struct Owner
{
  Control *control;

  explicit Owner(Control *input) : control(input)
  {
    if (control->references == 0)
      control->alive = 1;
    ++control->references;
  }

  Owner(const Owner& other) : control(other.control)
  {
    if (control->alive)
      ++control->references;
  }

  Owner(Owner&& other) : control(other.control)
  {
    other.control = 0;
  }

  ~Owner()
  {
    if (control && --control->references == 0)
      control->alive = 0;
  }
};

struct Result
{
  Owner member;

  explicit Result(Control *control) : member(control) {}
};

Control local_control = {0, 0, 5};
Control result_control = {0, 0, 7};

Owner choose(bool local)
{
  Owner base(&local_control);
  return local ? base : Result(&result_control).member;
}

int main()
{
  {
    Owner result = choose(false);
    if (!result.control->alive || result.control->value != 7)
      return 1;
  }

  Owner result = choose(true);
  return result.control->alive && result.control->value == 5 ? 0 : 2;
}
