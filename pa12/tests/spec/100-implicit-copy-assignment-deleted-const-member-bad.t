// VALIDATION: compile-fail
// N3485 focus: 12.8 [class.copy]
// Expected: an implicitly declared copy assignment is deleted for a const data
// member.

struct Holder
{
  const int value;
};

int main()
{
  Holder a = {1};
  const Holder b = {2};
  a = b;
  return 0;
}
