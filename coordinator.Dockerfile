FROM mcr.microsoft.com/dotnet/aspnet:5.0-alpine AS base
RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.tuna.tsinghua.edu.cn/g' /etc/apk/repositories
RUN apk add git
WORKDIR /app
EXPOSE 80
EXPOSE 443

FROM mcr.microsoft.com/dotnet/sdk:5.0-alpine AS build
WORKDIR /src
COPY 3rd_party /src/3rd_party
COPY coordinator/Karenia.Rurikawa.Coordinator.csproj coordinator/
RUN ls
RUN ls 3rd_party/SplitStream
WORKDIR /src/coordinator/
RUN dotnet restore
COPY coordinator/* /src/coordinator/
RUN dotnet build -c Release -o /app/build -v m

FROM build AS publish
RUN dotnet publish -c Release -o /app/publish

FROM base AS final
WORKDIR /app
COPY --from=publish /app/publish .
ENTRYPOINT ["dotnet", "Karenia.Rurikawa.Coordinator.dll"]
