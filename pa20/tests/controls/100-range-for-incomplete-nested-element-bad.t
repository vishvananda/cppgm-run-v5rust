// expect reject
struct Element;
extern Element elements[2][3];

int main()
{
  for (auto& row : elements) {}
}
