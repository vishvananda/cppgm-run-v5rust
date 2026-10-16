// N3485 focus: 11p4 [class.access] checks every name in a nested-name-specifier.
struct Owner {
private:
  struct Hidden { typedef int type; };
};
Owner::Hidden::type value;
