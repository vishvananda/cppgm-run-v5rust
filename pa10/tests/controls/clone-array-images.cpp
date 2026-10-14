// expect exit 0
int target;
const char * const names[4] = {"same", "same", nullptr};
int * const pointers[4] = {&target, nullptr, 0};
double zeros[3] = {-0.0, 1.5};
int main() {
    int first[3] = {1, 2};
    int second[3] = {1, 2};
    first[0] = 7;
    return first == second || first[0] != 7 || second[0] != 1 || second[2] != 0
        || names[0] != names[1] || names[2] != nullptr || names[3] != nullptr
        || pointers[0] != &target || pointers[1] != nullptr || pointers[3] != nullptr
        || zeros[1] != 1.5 || zeros[2] != 0.0;
}
