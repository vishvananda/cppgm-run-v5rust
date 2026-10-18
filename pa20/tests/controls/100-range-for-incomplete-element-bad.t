// expect reject
struct Element;
extern Element elements[2];

int main()
{
  for (auto& element : elements) {}
}
