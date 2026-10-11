// VALIDATION: compile-fail
// Static-member declaration legality is checked independently of storage demand.
struct X{static int n;
};
int X::n;
int X::n;
int main(){return 0;
}
