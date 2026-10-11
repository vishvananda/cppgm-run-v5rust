bool in_range(const double& value, const double& lo, const double& hi)
{
  return value < lo ? false : hi < value ? false : true;
}

int main()
{
  const float lo = 0.125;
  const float hi = 0.625;
  return in_range(lo + 0.01, lo, hi) ? 0 : 1;
}
