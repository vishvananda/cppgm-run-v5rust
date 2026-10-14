// Declaration order selects the second designated member; expect reject
union S {
    int first;
    int second;
    S() : second(1), first(2) {}
};
int main() { S local; return local.first; }
