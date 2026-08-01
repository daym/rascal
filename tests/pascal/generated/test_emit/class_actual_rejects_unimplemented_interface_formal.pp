unit u;
interface
{$interfaces corba}
type
  ireader = interface ['{11111111-1111-1111-1111-111111111111}']
    procedure next;
  end;
  iwriter = interface ['{22222222-2222-2222-2222-222222222222}']
    procedure write;
  end;
  treader = class(tobject, ireader)
    procedure next;
    procedure run;
  end;
procedure take(writer : iwriter);
implementation
procedure treader.next;
begin
end;
procedure take(writer : iwriter);
begin
end;
procedure treader.run;
begin
  take(self);
end;
end.
