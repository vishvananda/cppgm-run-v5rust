struct Period
{};

struct Date
{
  Date() {}
  Date(const Period &) {}

  int operator-(const Period &) const { return 11; }
};

int operator-(const Date &, const Date &)
{
  return 22;
}

int main()
{
  Date date;
  Period period;
  return date - period == 11 ? 0 : 1;
}
