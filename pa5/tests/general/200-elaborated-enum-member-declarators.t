enum kind { value };

struct owner {
  enum kind direct;
  enum kind* pointer;
};
