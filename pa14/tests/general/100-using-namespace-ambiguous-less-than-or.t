namespace ns {

template<typename T, typename U>
struct pair {
  T first;
  U second;
};

struct value {};

}

using namespace ns;

const pair<int, int> A = {1, 4};

bool f(int value)
{
  if(value < A.first || value > A.second) {
    return false;
  }
  return true;
}

int main()
{
  return f(3) ? 0 : 1;
}
