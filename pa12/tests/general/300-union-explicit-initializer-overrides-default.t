// VALIDATION: compile-pass
// N3485 focus: 12.6.2 [class.base.init] explicit union variant initialization

union Choice
{
  int first = 1;
  int second;

  Choice() : second(7) {}
};

int main()
{
  Choice choice;
  return choice.second == 7 ? 0 : 1;
}
