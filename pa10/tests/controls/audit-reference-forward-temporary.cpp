// expect exit 0
extern const int &r;
extern const int &s;
const int *p[3] = {&r, &s, &r};
const int &alias = r;
const int &r = 2;
const int &s = 3;
int x = 4;
extern int *const &pointer;
int *const *holder = &pointer;
int *const &pointer = &x;
extern const int &later;
const int *const &through = &later;
const int &later = 6;
const char *const &unused_string = "hi";
int main() {
    return *p[0] != 2 || *p[1] != 3 || p[0] != p[2] || &alias != &r
        || &r != p[0] || **holder != 4 || *through != 6;
}
