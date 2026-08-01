unit u;
interface
type
  tcollection = class
    procedure add(value : pointer);
  end;
procedure run(collection : tcollection);
implementation
procedure tcollection.add(value : pointer);
begin
end;
procedure run(collection : tcollection);
begin
  collection.add(pointer(5));
end;
end.
