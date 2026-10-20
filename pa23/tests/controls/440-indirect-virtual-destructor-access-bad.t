// Friendship with a direct base does not make the shared base's private
// destructor accessible from the most derived class; expect reject
struct V {
private:
    ~V() {}
    friend struct A;
};
struct A : virtual V { ~A() {} };
struct D : A {};
int main() { D object; }
