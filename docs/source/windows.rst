Windows Installation
====================

Shrimply builds natively on x86_64 Windows as a portable application. The
Windows build uses the Qt interface rather than the GTK interface used on
Linux, so it is a separate build with its own feature set.

An NVIDIA GPU and a current driver are required. The renderer is CUDA only,
and the packaged application loads ``nvcuda.dll`` from the driver; every other
library it needs is bundled.

Download a build
----------------

Windows builds are produced by the ``Windows`` GitHub Actions workflow rather
than published to the releases page. Open the `Actions tab
<https://github.com/soirihiroka/shrimply/actions/workflows/windows.yml>`__,
run the workflow, and download the ``shrimply-windows-x86_64`` artifact when it
finishes. The artifact contains ``shrimply-windows-x86_64.zip`` and a matching
``.sha256`` file.

Running the workflow from your own fork produces a build without a local
toolchain, which is the quickest way to get a Windows application.

Run it
------

Verify the download, extract it anywhere, and start the launcher:

.. code-block:: console

   > (Get-FileHash shrimply-windows-x86_64.zip -Algorithm SHA256).Hash
   > Expand-Archive shrimply-windows-x86_64.zip
   > .\shrimply-windows-x86_64\shrimply-qt.exe

The archive is self-contained, so no installation step is needed. Keep the
extracted directory intact; ``shrimply-qt.exe`` loads the Qt runtime and the
bundled libraries from alongside itself.

The launcher accepts one optional project path:

.. code-block:: console

   > .\shrimply-qt.exe C:\Users\you\videos\project.shrimp

Continue with :doc:`getting-started` to create a project and start editing.

Build from source
-----------------

Building on Windows needs, in addition to the requirements in
:doc:`development`:

* Visual Studio 2022 build tools, with the x64 MSVC environment loaded.
* The CUDA Toolkit 12.9, exposed as ``CUDA_PATH``.
* Qt 6.11.1 for ``win64_msvc2022_64``, with ``qmake.exe`` on ``PATH``.
* `vcpkg <https://github.com/microsoft/vcpkg>`__, supplying FFmpeg, OpenCV,
  Poppler, Pango, and Rubber Band. The manifests are in
  ``packaging/windows/vcpkg``.
* ``make``, ``ripgrep``, ``uv``, and LLVM for ``libclang``.
* Windows long path support, which the workspace exceeds without it.

The targets expect a Bash shell, such as the one shipped with Git for Windows:

.. code-block:: console

   $ make windows-check
   $ make windows-release
   $ make windows-package

``windows-check`` verifies formatting and source size, builds the CUDA
artifacts, then type-checks and lints the Qt binaries.
``windows-release`` builds the Qt launcher and editor, compiling the CUDA
kernels to PTX so the result is not tied to one GPU architecture.
``windows-package`` writes the portable archive to ``dist``.

``.github/workflows/windows.yml`` is the reference build. It pins every
dependency version and sets the environment the build expects, so consult it
when a local build cannot find a library.

Known limitations
-----------------

* The Windows application is built from the Qt interface, which does not yet
  cover every feature of the GTK interface used on Linux.
* MCP support is not packaged; ``shrimply-mcp`` is not included in the archive.
* AMD and Intel GPUs are not supported, on Windows or on any other platform.
