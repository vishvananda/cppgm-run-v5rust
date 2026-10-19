// expect exit 0
// N3485 8.5.4p7: a floating constant may narrow when within the target range.
int main()
{
  float sum{1.0 + 2.0};
  float fractional{1.0 / 3.0};
  double extended{1.25L * 2.0L};
  return sum == 3.0f && fractional > 0.3f && extended == 2.5 ? 0 : 1;
}
