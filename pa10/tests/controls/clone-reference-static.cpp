// expect exit 0
namespace owner {
    int data[3] = {2, 3, 4};
    int &element = data[1];
    int &chain = element;
    const int &temporary = 7;
    const int &same = temporary;
    const int *address = &same;
}
int main() {
    owner::chain = 8;
    return owner::data[1] != 8 || owner::temporary != 7 || owner::address != &owner::temporary;
}
