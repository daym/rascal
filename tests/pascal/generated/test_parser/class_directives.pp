unit u;
interface
type
  TFoo = class
    procedure A; dynamic;
    procedure B(x:integer); overload;
    procedure B(x:string);  overload;
    procedure C; reintroduce;
    class procedure Classy;
    class function  ClassyF: integer;
  end;
implementation
end.
