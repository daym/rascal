unit u;
interface
type
  tnode = class
  end;
  tcallparanode = class(tnode)
    right : tnode;
  end;
  pcallparanode = ^tcallparanode;
procedure demo(pt : tcallparanode);
implementation
procedure demo(pt : tcallparanode);
var
  oldppt : pcallparanode;
begin
  oldppt := @pt.right;
end;
end.
