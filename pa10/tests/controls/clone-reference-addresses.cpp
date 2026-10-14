// expect exit 0
int data[2] = {3, 4};
int calls;
int (&array())[2] { ++calls; return data; }
int &choose(bool condition, int &left, int &right) { return condition ? left : right; }
int bump(int x) { return x + 1; }
int main() {
    int left = 2;
    int right = 3;
    int &selected = choose(true, left, right);
    selected = array()[1];
    int (&function)(int) = bump;
    int (&refarr)[2] = data;
    int *p = &refarr[0];
    int *const &pointer = p;
    const int &readonly = left;
    const_cast<int &>(readonly) += 1;
    return function(left) != 6 || pointer[1] != 4 || right != 3 || calls != 1;
}
