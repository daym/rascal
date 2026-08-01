unit u;
interface
type
  tfoo = class
  strict protected
    class procedure hidden;
    class procedure hook; virtual;
  public
    class procedure run;
  end;
implementation
class procedure tfoo.hidden;
begin
end;
class procedure tfoo.hook;
begin
end;
class procedure tfoo.run;
begin
end;
end.
