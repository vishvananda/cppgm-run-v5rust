namespace source {
int value = 4;
}

namespace target {
static constexpr int &alias = source::value;
}

int main()
{
  target::alias = 7;
  return source::value;
}
